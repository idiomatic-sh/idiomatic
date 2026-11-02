//! Conservatively fuse exclusively source-keyed closure consumers into physical demand joins.

use ryo_language::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, Operand, ReachLookup, RepeatedRows, StepId, Test,
};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, PlanStep, StepScope};

/// One closure and the pure projection map through which a consumer sees its logical RHS.
#[derive(Clone)]
struct Rooted {
    /// Original closure step; all of its remaining consumers must qualify together.
    root: usize,
    /// Physical complete/pure graph input.
    edges: Input,
    /// Endpoint/partition metadata and logical output map.
    reach: ReachLookup,
}

/// Borrowed physical rewrite inputs; no provider, row population or mutable runtime state.
pub(crate) struct DemandClosure<'a> {
    /// Typed steps after ordinary pruning/column rewriting.
    steps: &'a [PlanStep],
    /// Exported/frontier steps must retain ordinary closure semantics.
    protected: &'a [StepId],
}

impl<'a> DemandClosure<'a> {
    /// Protect explicit outputs before considering any destructive rewrite.
    pub(crate) fn new(steps: &'a [PlanStep], protected: &'a [StepId]) -> Self {
        Self { steps, protected }
    }

    /// All keyed consumers lower together, or the original closure remains unchanged.
    pub(crate) fn rewrite(&self) -> Vec<PlanStep> {
        let mut rooted: Vec<Option<Rooted>> = Vec::with_capacity(self.steps.len());
        for (index, step) in self.steps.iter().enumerate() {
            rooted.push(self.root(index, step, &rooted));
        }
        let candidates: Vec<_> = self
            .steps
            .iter()
            .map(|step| candidate(step, &rooted))
            .collect();
        let mut safe = vec![true; self.steps.len()];
        for protected in self.protected {
            if let Some(root) = rooted.get(protected.0 as usize).and_then(Option::as_ref) {
                safe[root.root] = false;
            }
        }
        for (index, step) in self.steps.iter().enumerate() {
            for input in named(&inputs_of(&step.operation)) {
                let Input::Step(id) = input else {
                    continue;
                };
                let Some(root) = rooted.get(id.0 as usize).and_then(Option::as_ref) else {
                    continue;
                };
                let wrapper = rooted[index]
                    .as_ref()
                    .is_some_and(|child| child.root == root.root && index != root.root);
                let keyed = candidates[index]
                    .as_ref()
                    .is_some_and(|child| child.root == root.root)
                    && matches!(&step.operation, Operation::Join { right, .. } if right == input);
                if !wrapper && !keyed {
                    safe[root.root] = false;
                }
            }
        }
        self.steps
            .iter()
            .zip(candidates)
            .map(|(step, rooted)| {
                let Some(rooted) = rooted.filter(|root| safe[root.root]) else {
                    return step.clone();
                };
                let Operation::Join {
                    left,
                    left_on,
                    right_on,
                    output,
                    filter,
                    view,
                    ..
                } = &step.operation
                else {
                    return step.clone();
                };
                PlanStep {
                    scope: step.scope,
                    operation: Operation::Join {
                        left: *left,
                        right: rooted.edges,
                        left_on: left_on.clone(),
                        right_on: right_on.clone(),
                        output: output.clone(),
                        filter: filter.clone(),
                        view: view.clone(),
                        reach: Some(rooted.reach),
                    },
                }
            })
            .collect()
    }

    /// Resolve only ordinary closure and non-distinct, predicate-free column wrappers.
    fn root(&self, index: usize, step: &PlanStep, before: &[Option<Rooted>]) -> Option<Rooted> {
        if step.scope != StepScope::PerFile {
            return None;
        }
        match &step.operation {
            Operation::Closure {
                edges,
                source,
                destination,
                keys,
            } => {
                let (edges, source, destination, keys) =
                    graph_source(*edges, *source, *destination, keys, self.steps)?;
                Some(Rooted {
                    root: index,
                    edges,
                    reach: ReachLookup {
                        source,
                        destination,
                        keys: keys.clone(),
                        columns: (0..keys.len() + 2)
                            .map(|column| ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX)))
                            .collect(),
                    },
                })
            }
            Operation::Project {
                from: Input::Step(id),
                columns,
                distinct: RepeatedRows::Kept,
            } => project(before.get(id.0 as usize)?.as_ref()?, columns, false),
            Operation::Select {
                from: Input::Step(id),
                comparisons,
                kept,
            } if comparisons.is_empty() => {
                project(before.get(id.0 as usize)?.as_ref()?, kept, true)
            }
            Operation::Select { .. }
            | Operation::Project { .. }
            | Operation::Extend { .. }
            | Operation::Join { .. }
            | Operation::Union { .. }
            | Operation::Intersection { .. }
            | Operation::Difference { .. }
            | Operation::Antijoin { .. }
            | Operation::GroupCount { .. }
            | Operation::GroupDigest { .. }
            | Operation::Capture { .. }
            | Operation::Rank { .. }
            | Operation::Replace { .. }
            | Operation::OrderedMatch { .. }
            | Operation::Words { .. }
            | Operation::Axis { .. }
            | Operation::OrderedDfs { .. }
            | Operation::Cycles { .. }
            | Operation::Rows { .. }
            | Operation::Hosted { .. }
            | Operation::SpanBounds { .. }
            | Operation::SourceMatches { .. }
            | Operation::SpanBetween { .. } => None,
        }
    }
}

/// Typed equality binds every partition field and the logical closure source; extra keys retain residual checks.
fn candidate(step: &PlanStep, rooted: &[Option<Rooted>]) -> Option<Rooted> {
    let Operation::Join {
        left,
        right: Input::Step(id),
        right_on,
        filter,
        view,
        reach: None,
        ..
    } = &step.operation
    else {
        return None;
    };
    if step.scope != StepScope::PerFile || !filter.is_empty() || !view.is_empty() {
        return None;
    }
    let root = rooted.get(id.0 as usize)?.as_ref()?;
    if let Input::Step(left) = left
        && rooted
            .get(left.0 as usize)
            .and_then(Option::as_ref)
            .is_some_and(|other| other.root == root.root)
    {
        return None;
    }
    let bound: Vec<_> = right_on
        .iter()
        .map(|column| root.reach.columns.get(column.0 as usize).copied())
        .collect::<Option<_>>()?;
    (0..=root.reach.keys.len())
        .all(|column| bound.contains(&ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX))))
        .then(|| root.clone())
}

/// Empty Select keeps every column; empty Project keeps no columns.
fn project(root: &Rooted, columns: &[ColumnIndex], empty_identity: bool) -> Option<Rooted> {
    let mut result = root.clone();
    if !empty_identity || !columns.is_empty() {
        result.reach.columns = columns
            .iter()
            .map(|column| root.reach.columns.get(column.0 as usize).copied())
            .collect::<Option<_>>()?;
    }
    Some(result)
}

/// Positive closure ignores parallel duplicate edges. Pull input views through only when every
/// projected physical column is still validated by an endpoint/partition check. This is NOT a
/// generic Project/Axis rewrite: direct axes and outer closure projections retain their bags.
fn graph_source(
    mut input: Input,
    source: ColumnIndex,
    destination: ColumnIndex,
    keys: &[ColumnIndex],
    steps: &[PlanStep],
) -> Option<(Input, ColumnIndex, ColumnIndex, Vec<ColumnIndex>)> {
    let mut used = keys.to_vec();
    used.extend([source, destination]);
    for _ in 0..=steps.len() {
        let Input::Step(id) = input else {
            break;
        };
        let step = steps.get(id.0 as usize)?;
        if step.scope != StepScope::PerFile {
            return None;
        }
        let view = match &step.operation {
            Operation::Project { from, columns, .. } => Some((*from, columns.as_slice())),
            Operation::Select {
                from,
                comparisons,
                kept,
            } if comparisons.is_empty() => Some((*from, kept.as_slice())),
            Operation::Select { .. }
            | Operation::Extend { .. }
            | Operation::Join { .. }
            | Operation::Union { .. }
            | Operation::Intersection { .. }
            | Operation::Difference { .. }
            | Operation::Antijoin { .. }
            | Operation::GroupCount { .. }
            | Operation::GroupDigest { .. }
            | Operation::Capture { .. }
            | Operation::Rank { .. }
            | Operation::Replace { .. }
            | Operation::OrderedMatch { .. }
            | Operation::Words { .. }
            | Operation::Axis { .. }
            | Operation::Closure { .. }
            | Operation::OrderedDfs { .. }
            | Operation::Cycles { .. }
            | Operation::Rows { .. }
            | Operation::Hosted { .. }
            | Operation::SpanBounds { .. }
            | Operation::SourceMatches { .. }
            | Operation::SpanBetween { .. } => None,
        };
        let Some((from, columns)) = view else {
            break;
        };
        if columns.is_empty() && matches!(step.operation, Operation::Select { .. }) {
            input = from;
            continue;
        }
        let mapped = used
            .iter()
            .map(|column| columns.get(column.0 as usize).copied())
            .collect::<Option<Vec<_>>>()?;
        if columns.iter().any(|column| !mapped.contains(column)) {
            break;
        }
        used = mapped;
        input = from;
    }
    if !graph_input(input, steps) {
        return None;
    }
    let destination = used.pop()?;
    let source = used.pop()?;
    Some((input, source, destination, used))
}

/// Stored relations and the already supported pure selection/projection chains alone qualify.
fn graph_input(mut input: Input, steps: &[PlanStep]) -> bool {
    for _ in 0..=steps.len() {
        match input {
            Input::Relation(Relation::SyntaxTree | Relation::SyntaxChild) => return false,
            Input::Relation(_) => return true,
            Input::Step(id) => {
                let Some(step) = steps
                    .get(id.0 as usize)
                    .filter(|step| step.scope == StepScope::PerFile)
                else {
                    return false;
                };
                input = match &step.operation {
                    Operation::Project {
                        from,
                        distinct: RepeatedRows::Kept,
                        ..
                    } => *from,
                    Operation::Select {
                        from, comparisons, ..
                    } if comparisons.iter().all(pure) => *from,
                    Operation::Select { .. }
                    | Operation::Project { .. }
                    | Operation::Extend { .. }
                    | Operation::Join { .. }
                    | Operation::Union { .. }
                    | Operation::Intersection { .. }
                    | Operation::Difference { .. }
                    | Operation::Antijoin { .. }
                    | Operation::GroupCount { .. }
                    | Operation::GroupDigest { .. }
                    | Operation::Capture { .. }
                    | Operation::Rank { .. }
                    | Operation::Replace { .. }
                    | Operation::OrderedMatch { .. }
                    | Operation::Words { .. }
                    | Operation::Axis { .. }
                    | Operation::Closure { .. }
                    | Operation::OrderedDfs { .. }
                    | Operation::Cycles { .. }
                    | Operation::Rows { .. }
                    | Operation::Hosted { .. }
                    | Operation::SpanBounds { .. }
                    | Operation::SourceMatches { .. }
                    | Operation::SpanBetween { .. } => return false,
                };
            }
        }
    }
    false
}

/// Error-sensitive functions/patterns stay in their original owner-local execution path.
fn pure(comparison: &ryo_values::domain::plan_parts::types::Comparison) -> bool {
    !matches!(comparison.left, Operand::Function(_))
        && !matches!(comparison.right, Operand::Function(_))
        && matches!(
            comparison.test,
            Test::Equal
                | Test::NotEqual
                | Test::LessThan
                | Test::LessThanOrEqual
                | Test::GreaterThan
                | Test::GreaterThanOrEqual
                | Test::StartsWith
                | Test::EndsWith
                | Test::In
                | Test::NotIn
        )
}
