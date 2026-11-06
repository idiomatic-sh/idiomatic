//! Conservatively fuse exclusively source-keyed closure consumers into physical demand joins.

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, Operand, ReachLookup, RepeatedRows, StepId, Test,
};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::plan_parts::types::StepScope;

/// Which step of the plan a closure sits at.
#[derive(Clone, Copy, PartialEq, Eq)]
struct RootStep(usize);

/// What an empty list of columns means to the step that holds it.
#[derive(Clone, Copy)]
enum EmptyColumns {
    /// An empty select keeps every column.
    KeepsEvery,
    /// An empty project keeps no column.
    KeepsNone,
}

/// Whether a chain of steps ends at a stored relation the closure can be pulled to.
#[derive(Clone, Copy)]
enum GraphInput {
    /// A stored relation, or a pure chain over one.
    Stored,
    /// Anything else.
    Unsupported,
}

/// Whether a comparison is safe to evaluate in another place.
#[derive(Clone, Copy)]
enum Purity {
    /// Plain values and plain tests.
    Pure,
    /// A function or pattern that can fail.
    ErrorSensitive,
}

/// The graph a closure reads, with its endpoints and partition keys, after pulling through views.
struct GraphSource {
    /// The pulled-through input.
    input: Input,
    /// The source endpoint column.
    source: ColumnIndex,
    /// The destination endpoint column.
    destination: ColumnIndex,
    /// The partition keys.
    keys: Vec<ColumnIndex>,
}

/// One closure and the pure projection map through which a consumer sees its logical RHS.
#[derive(Clone)]
struct Rooted {
    /// Original closure step; all of its remaining consumers must qualify together.
    root: RootStep,
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
            rooted.push(self.root(RootStep(index), step, &rooted));
        }
        let candidates: Vec<_> = self
            .steps
            .iter()
            .map(|step| candidate(step, &rooted))
            .collect();
        let mut safe = vec![true; self.steps.len()];
        for protected in self.protected {
            if let Some(root) = rooted.get(usize::try_from(protected.0).unwrap_or(usize::MAX)).and_then(Option::as_ref) {
                safe[root.root.0] = false;
            }
        }
        for (index, step) in self.steps.iter().enumerate() {
            for input in named(&inputs_of(&step.operation)) {
                let Input::Step(id) = input else {
                    continue;
                };
                let Some(root) = rooted.get(usize::try_from(id.0).unwrap_or(usize::MAX)).and_then(Option::as_ref) else {
                    continue;
                };
                let wrapper = rooted[index]
                    .as_ref()
                    .is_some_and(|child| child.root == root.root && index != root.root.0);
                let keyed = candidates[index]
                    .as_ref()
                    .is_some_and(|child| child.root == root.root)
                    && matches!(&step.operation, Operation::Join { right, .. } if right == input);
                if !wrapper && !keyed {
                    safe[root.root.0] = false;
                }
            }
        }
        self.steps
            .iter()
            .zip(candidates)
            .map(|(step, rooted)| {
                let Some(rooted) = rooted.filter(|root| safe[root.root.0]) else {
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
    fn root(&self, index: RootStep, step: &PlanStep, before: &[Option<Rooted>]) -> Option<Rooted> {
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
                let GraphSource {
                    input: edges,
                    source,
                    destination,
                    keys,
                } = graph_source(*edges, *source, *destination, keys, self.steps)?;
                Some(Rooted {
                    root: index,
                    edges,
                    reach: ReachLookup {
                        source,
                        destination,
                        keys: keys.clone(),
                        columns: std::iter::successors(Some(ColumnIndex(0)), |previous| {
                            Some(ColumnIndex(previous.0.saturating_add(1)))
                        })
                        .take(keys.len() + 2)
                        .collect(),
                    },
                })
            }
            Operation::Project {
                from: Input::Step(id),
                columns,
                distinct: RepeatedRows::Kept,
            } => project(before.get(usize::try_from(id.0).unwrap_or(usize::MAX))?.as_ref()?, columns, EmptyColumns::KeepsNone),
            Operation::Select {
                from: Input::Step(id),
                comparisons,
                kept,
            } if comparisons.is_empty() => project(before.get(usize::try_from(id.0).unwrap_or(usize::MAX))?.as_ref()?, kept, EmptyColumns::KeepsEvery),
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
            | Operation::RenderSyntax { .. }
            | Operation::FitsRendered { .. }
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
    let root = rooted.get(usize::try_from(id.0).unwrap_or(usize::MAX))?.as_ref()?;
    if let Input::Step(left) = left
        && rooted
            .get(usize::try_from(left.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .is_some_and(|other| other.root == root.root)
    {
        return None;
    }
    let bound: Vec<_> = right_on
        .iter()
        .map(|column| root.reach.columns.get(usize::try_from(column.0).unwrap_or(usize::MAX)).copied())
        .collect::<Option<_>>()?;
    (0..=root.reach.keys.len())
        .all(|column| bound.contains(&ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX))))
        .then(|| root.clone())
}

/// Empty Select keeps every column; empty Project keeps no columns.
fn project(root: &Rooted, columns: &[ColumnIndex], empty: EmptyColumns) -> Option<Rooted> {
    let mut result = root.clone();
    if matches!(empty, EmptyColumns::KeepsNone) || !columns.is_empty() {
        result.reach.columns = columns
            .iter()
            .map(|column| root.reach.columns.get(usize::try_from(column.0).unwrap_or(usize::MAX)).copied())
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
) -> Option<GraphSource> {
    let mut used = keys.to_vec();
    used.extend([source, destination]);
    for _ in 0..=steps.len() {
        let Input::Step(id) = input else {
            break;
        };
        let step = steps.get(usize::try_from(id.0).unwrap_or(usize::MAX))?;
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
            | Operation::RenderSyntax { .. }
            | Operation::FitsRendered { .. }
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
            .map(|column| columns.get(usize::try_from(column.0).unwrap_or(usize::MAX)).copied())
            .collect::<Option<Vec<_>>>()?;
        if columns.iter().any(|column| !mapped.contains(column)) {
            break;
        }
        used = mapped;
        input = from;
    }
    if matches!(graph_input(input, steps), GraphInput::Unsupported) {
        return None;
    }
    let destination = used.pop()?;
    let source = used.pop()?;
    Some(GraphSource {
        input,
        source,
        destination,
        keys: used,
    })
}

/// Stored relations and the already supported pure selection/projection chains alone qualify.
fn graph_input(mut input: Input, steps: &[PlanStep]) -> GraphInput {
    for _ in 0..=steps.len() {
        match input {
            Input::Relation(Relation::SyntaxTree | Relation::SyntaxChild)
            | Input::BoundObservation(_) => return GraphInput::Unsupported,
            Input::Relation(_) => return GraphInput::Stored,
            Input::Step(id) => {
                let Some(step) = steps
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                    .filter(|step| step.scope == StepScope::PerFile)
                else {
                    return GraphInput::Unsupported;
                };
                input = match &step.operation {
                    Operation::Project {
                        from,
                        distinct: RepeatedRows::Kept,
                        ..
                    } => *from,
                    Operation::Select {
                        from, comparisons, ..
                    } if comparisons.iter().all(|comparison| matches!(purity(comparison), Purity::Pure)) => *from,
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
                    | Operation::RenderSyntax { .. }
                    | Operation::FitsRendered { .. }
                    | Operation::SpanBetween { .. } => return GraphInput::Unsupported,
                };
            }
        }
    }
    GraphInput::Unsupported
}

/// Error-sensitive functions/patterns stay in their original owner-local execution path.
fn purity(comparison: &ryo_values::domain::plan_parts::types::PlanComparison) -> Purity {
    let plain = !matches!(comparison.left, Operand::Function(_))
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
        );
    if plain {
        Purity::Pure
    } else {
        Purity::ErrorSensitive
    }
}
