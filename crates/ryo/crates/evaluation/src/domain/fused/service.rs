//! Fusing the plans of a request: their steps are interned, so a step two rules (or two imports of one fragment) both make is one
//! step, and the selects over the parser's tree are indexed by the kinds they keep.

use super::errors::FusedError;
use super::types::{Admits, Fused, NodeId, Root};
use ryo_relations::domain::relations::types::{KindBits, Relation};
use ryo_values::domain::plan_parts::types::{Input, Operand, QueryLiteral, StepId, Test};
use ryo_values::domain::plans::types::{Operation, Plan};

/// An operation as a key: plans hold no floating value, so equality is total on them.
#[derive(Debug, PartialEq)]
struct Key(Operation);
impl Eq for Key {}
#[expect(
    clippy::derived_hash_with_manual_eq,
    reason = "Eq only asserts the derived PartialEq is total, which it is over plans that hold no float"
)]
impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// The inputs an operation reads, to rewrite; `None` names an operation the spike does not evaluate.
fn inputs_mut(operation: &mut Operation) -> Option<Vec<&mut Input>> {
    Some(match operation {
        Operation::Select { from, .. }
        | Operation::Project { from, .. }
        | Operation::Extend { from, .. }
        | Operation::GroupCount { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Replace { from, .. }
        | Operation::SourceMatches { from, .. }
        | Operation::SpanBounds { from, .. } => vec![from],
        Operation::Join {
            left, right, reach, ..
        } => {
            if reach.is_some() {
                return None;
            }
            vec![left, right]
        }
        Operation::Union { left, right }
        | Operation::Intersection { left, right }
        | Operation::Difference { left, right }
        | Operation::Antijoin { left, right, .. }
        | Operation::OrderedMatch { left, right, .. } => vec![left, right],
        Operation::Closure { edges, .. } => vec![edges],
        Operation::GroupDigest { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. } => return None,
    })
}

/// The name of an operation, for a refusal.
fn name_of(operation: &Operation) -> &'static str {
    match operation {
        Operation::Select { .. } => "select",
        Operation::Project { .. } => "project",
        Operation::Extend { .. } => "extend",
        Operation::Join { .. } => "join",
        Operation::Union { .. } => "union",
        Operation::Intersection { .. } => "intersection",
        Operation::Difference { .. } => "difference",
        Operation::Antijoin { .. } => "antijoin",
        Operation::GroupCount { .. } => "group-count",
        Operation::GroupDigest { .. } => "group-digest",
        Operation::Capture { .. } => "capture",
        Operation::Rank { .. } => "rank",
        Operation::Replace { .. } => "replace",
        Operation::OrderedMatch { .. } => "ordered-match",
        Operation::Words { .. } => "words",
        Operation::Axis { .. } => "axis",
        Operation::Closure { .. } => "closure",
        Operation::OrderedDfs { .. } => "ordered-dfs",
        Operation::Cycles { .. } => "cycles",
        Operation::Rows { .. } => "rows",
        Operation::Hosted { .. } => "hosted",
        Operation::SpanBetween { .. } => "span-between",
        Operation::SourceMatches { .. } => "source-matches",
        Operation::RenderSyntax { .. } => "render-syntax",
        Operation::FitsRendered { .. } => "fits-rendered",
        Operation::SpanBounds { .. } => "span-bounds",
    }
}

/// The kinds a select of the parser's tree may keep: the kind its comparisons name equal, when one does.
fn admits(operation: &Operation, kinds: &[&str]) -> Admits {
    let Operation::Select { comparisons, .. } = operation else {
        return Admits::Every;
    };
    for comparison in comparisons {
        let named = match (&comparison.left, &comparison.right) {
            (Operand::Column(column), Operand::Literal(QueryLiteral::Text(text)))
            | (Operand::Literal(QueryLiteral::Text(text)), Operand::Column(column))
                if column.0 == 1 && comparison.test == Test::Equal =>
            {
                Some(&*text.0)
            }
            _ => None,
        };
        if let Some(named) = named {
            return Admits::Only(
                kinds
                    .iter()
                    .enumerate()
                    .filter(|(_, kind)| **kind == named)
                    .filter_map(|(bits, _)| u16::try_from(bits).ok().map(KindBits))
                    .collect(),
            );
        }
    }
    Admits::Every
}

/// Fuses `plans`, one for each rule in rule order, over a language whose kinds are named `kinds` by their numbers.
///
/// # Errors
///
/// `Parameters` for a rule with parameters, `Unsupported` for a step the spike does not evaluate.
pub fn fuse(plans: &[Plan], kinds: &[&str]) -> Result<Fused, FusedError> {
    let mut nodes: Vec<Operation> = Vec::new();
    let mut known: hashbrown::HashMap<Key, NodeId> = hashbrown::HashMap::new();
    let mut emits = Vec::with_capacity(plans.len());
    for (rule, plan) in plans.iter().enumerate() {
        if !plan.parameters.is_empty() {
            return Err(FusedError::Parameters { rule });
        }
        let mut local: Vec<NodeId> = Vec::with_capacity(plan.steps.len());
        for step in &plan.steps {
            let mut operation = step.operation.clone();
            let name = name_of(&operation);
            for input in inputs_mut(&mut operation).ok_or(FusedError::Unsupported {
                rule,
                operation: name,
            })? {
                match input {
                    Input::Step(id) => {
                        let node = local
                            .get(usize::from(*id))
                            .ok_or(FusedError::Unsupported {
                                rule,
                                operation: "forward step",
                            })?;
                        *id = StepId(u32::try_from(node.0).unwrap_or(u32::MAX));
                    }
                    Input::Relation(_) => {}
                    Input::BoundObservation(_) => {
                        return Err(FusedError::Unsupported {
                            rule,
                            operation: "bound observation",
                        });
                    }
                }
            }
            let node = *known.entry(Key(operation.clone())).or_insert_with(|| {
                nodes.push(operation);
                NodeId(nodes.len() - 1)
            });
            local.push(node);
        }
        emits.push(
            *local
                .get(usize::from(plan.emit.step))
                .ok_or(FusedError::Unsupported {
                    rule,
                    operation: "emit",
                })?,
        );
    }
    let roots: Vec<Root> = nodes
        .iter()
        .enumerate()
        .filter(|(_, operation)| {
            matches!(
                operation,
                Operation::Select {
                    from: Input::Relation(Relation::SyntaxTree),
                    ..
                }
            )
        })
        .map(|(node, operation)| Root {
            node: NodeId(node),
            admits: admits(operation, kinds),
        })
        .collect();
    let mut by_kind = vec![Vec::new(); kinds.len()];
    let mut every = Vec::new();
    for (place, root) in roots.iter().enumerate() {
        match &root.admits {
            Admits::Every => every.push(place),
            Admits::Only(only) => {
                for kind in only {
                    if let Some(each) = by_kind.get_mut(usize::from(kind.0)) {
                        each.push(place);
                    }
                }
            }
        }
    }
    Ok(Fused {
        nodes,
        emits,
        roots,
        by_kind,
        every,
    })
}
