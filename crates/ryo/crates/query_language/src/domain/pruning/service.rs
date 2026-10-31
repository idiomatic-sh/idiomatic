//! The `pruning` service: free functions named by verb over the steps nothing reads, dropped, and the inputs walked.

use crate::domain::rewriting::types::Planned;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// Gives `visit` each input the operation reads.
pub(crate) fn each_input(operation: &mut Operation, visit: &mut dyn FnMut(&mut Input)) {
    match operation {
        Operation::Replace { from, .. }
        | Operation::Select { from, .. }
        | Operation::Project { from, .. }
        | Operation::Extend { from, .. }
        | Operation::GroupCount { from, .. }
        | Operation::GroupDigest { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Rank { from, .. }
        | Operation::Words { from, .. } => visit(from),
        Operation::OrderedMatch { left, right, .. }
        | Operation::Join { left, right, .. }
        | Operation::Union { left, right }
        | Operation::Intersection { left, right }
        | Operation::Difference { left, right }
        | Operation::Antijoin { left, right, .. } => {
            visit(left);
            visit(right);
        }
        Operation::Axis {
            from,
            edges,
            domain,
            ..
        } => {
            visit(from);
            visit(edges);
            visit(domain);
        }
        Operation::Closure { edges, .. } | Operation::Cycles { edges, .. } => visit(edges),
        Operation::Rows { .. } | Operation::Hosted { .. } => {}
    }
}

/// The plan without the steps nothing reads, which the idioms leave behind, and the place the
/// emitted step has in what is left. Steps keep their order, and each step's inputs follow.
#[must_use]
pub(crate) fn without_unused(steps: Vec<PlanStep>, emitted: StepId) -> Planned {
    let mut wanted = vec![false; steps.len()];
    let mut pending = vec![emitted];
    let mut reading = steps.clone();
    while let Some(id) = pending.pop() {
        let place = usize::try_from(id.0).unwrap_or(usize::MAX);
        #[expect(
            clippy::indexing_slicing,
            reason = "a step reads only steps of its plan; a read past them is a bug that must crash, not a step dropped from the plan"
        )]
        let (seen, step) = (&mut wanted[place], &mut reading[place]);
        if *seen {
            continue;
        }
        *seen = true;
        each_input(&mut step.operation, &mut |input| {
            if let Input::Step(read) = input {
                pending.push(*read);
            }
        });
    }
    let mut renumbered = vec![StepId(0); steps.len()];
    let mut next = 0_u32;
    for (slot, keep) in renumbered.iter_mut().zip(&wanted) {
        if *keep {
            *slot = StepId(next);
            next += 1;
        }
    }
    let kept = steps
        .into_iter()
        .zip(&wanted)
        .filter(|(_, keep)| **keep)
        .map(|(mut step, _)| {
            each_input(&mut step.operation, &mut |input| {
                if let Input::Step(read) = input {
                    *read = kept_as(&renumbered, *read);
                }
            });
            step
        })
        .collect();
    Planned {
        steps: kept,
        emitted: kept_as(&renumbered, emitted),
    }
}

/// The id the step `id` keeps once the steps nothing reads are dropped.
#[expect(
    clippy::indexing_slicing,
    reason = "a kept step reads only steps of its plan, and the emitted step is one; another id is a bug that must crash, not an input left pointing at a dropped step"
)]
fn kept_as(renumbered: &[StepId], id: StepId) -> StepId {
    renumbered[usize::try_from(id.0).unwrap_or(usize::MAX)]
}
