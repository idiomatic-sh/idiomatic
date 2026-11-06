//! Conservatively fuse exclusively source-keyed closure consumers into physical demand joins.

use crate::domain::fusing::types::EmptyColumns;
use crate::domain::fusing::types::RootStep;
use crate::domain::fusing::types::pulled_graph::value::PulledGraph;
use crate::domain::fusing::types::pulled_graph::value::graph_source;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::ReachLookup;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::service::inputs_of;
use ryo_values::domain::plans::service::named;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

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
            if let Some(root) = rooted
                .get(usize::try_from(protected.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
            {
                safe[root.root.0] = false;
            }
        }
        for (index, step) in self.steps.iter().enumerate() {
            for input in named(&inputs_of(&step.operation)) {
                let Input::Step(id) = input else {
                    continue;
                };
                let Some(root) = rooted
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                    .and_then(Option::as_ref)
                else {
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
                let PulledGraph {
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
            } => project(
                before
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))?
                    .as_ref()?,
                columns,
                EmptyColumns::KeepsNone,
            ),
            Operation::Select {
                from: Input::Step(id),
                comparisons,
                kept,
            } if comparisons.is_empty() => project(
                before
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))?
                    .as_ref()?,
                kept,
                EmptyColumns::KeepsEvery,
            ),
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
    let root = rooted
        .get(usize::try_from(id.0).unwrap_or(usize::MAX))?
        .as_ref()?;
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
        .map(|column| {
            root.reach
                .columns
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .copied()
        })
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
            .map(|column| {
                root.reach
                    .columns
                    .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                    .copied()
            })
            .collect::<Option<_>>()?;
    }
    Some(result)
}
