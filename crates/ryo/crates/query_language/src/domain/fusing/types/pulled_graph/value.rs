//! Where a closure's graph is read from, pulled through the views above it.

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// Whether a chain of steps ends at a stored relation the closure can be pulled to.
#[derive(Clone, Copy)]
pub(crate) enum GraphInputKind {
    /// A stored relation, or a pure chain over one.
    Stored,
    /// Anything else.
    Unsupported,
}

/// Whether a comparison is safe to evaluate in another place.
#[derive(Clone, Copy)]
pub(crate) enum Purity {
    /// Plain values and plain tests.
    Pure,
    /// A function or pattern that can fail.
    ErrorSensitive,
}

/// The graph a closure reads, with its endpoints and partition keys, after pulling through views.
pub(crate) struct PulledGraph {
    /// The pulled-through input.
    pub input: Input,
    /// The source endpoint column.
    pub source: ColumnIndex,
    /// The destination endpoint column.
    pub destination: ColumnIndex,
    /// The partition keys.
    pub keys: Vec<ColumnIndex>,
}

/// Positive closure ignores parallel duplicate edges. Pull input views through only when every
/// projected physical column is still validated by an endpoint/partition check. This is NOT a
/// generic Project/Axis rewrite: direct axes and outer closure projections retain their bags.
pub(crate) fn graph_source(
    mut input: Input,
    source: ColumnIndex,
    destination: ColumnIndex,
    keys: &[ColumnIndex],
    steps: &[PlanStep],
) -> Option<PulledGraph> {
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
            .map(|column| {
                columns
                    .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                    .copied()
            })
            .collect::<Option<Vec<_>>>()?;
        if columns.iter().any(|column| !mapped.contains(column)) {
            break;
        }
        used = mapped;
        input = from;
    }
    if matches!(graph_input(input, steps), GraphInputKind::Unsupported) {
        return None;
    }
    let destination = used.pop()?;
    let source = used.pop()?;
    Some(PulledGraph {
        input,
        source,
        destination,
        keys: used,
    })
}

/// Stored relations and the already supported pure selection/projection chains alone qualify.
fn graph_input(mut input: Input, steps: &[PlanStep]) -> GraphInputKind {
    for _ in 0..=steps.len() {
        match input {
            Input::Relation(Relation::SyntaxTree | Relation::SyntaxChild)
            | Input::BoundObservation(_) => return GraphInputKind::Unsupported,
            Input::Relation(_) => return GraphInputKind::Stored,
            Input::Step(id) => {
                let Some(step) = steps
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                    .filter(|step| step.scope == StepScope::PerFile)
                else {
                    return GraphInputKind::Unsupported;
                };
                input = match &step.operation {
                    Operation::Project {
                        from,
                        distinct: RepeatedRows::Kept,
                        ..
                    } => *from,
                    Operation::Select {
                        from, comparisons, ..
                    } if comparisons
                        .iter()
                        .all(|comparison| matches!(purity(comparison), Purity::Pure)) =>
                    {
                        *from
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
                    | Operation::Closure { .. }
                    | Operation::OrderedDfs { .. }
                    | Operation::Cycles { .. }
                    | Operation::Rows { .. }
                    | Operation::Hosted { .. }
                    | Operation::SpanBounds { .. }
                    | Operation::SourceMatches { .. }
                    | Operation::RenderSyntax { .. }
                    | Operation::FitsRendered { .. }
                    | Operation::SpanBetween { .. } => return GraphInputKind::Unsupported,
                };
            }
        }
    }
    GraphInputKind::Unsupported
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
