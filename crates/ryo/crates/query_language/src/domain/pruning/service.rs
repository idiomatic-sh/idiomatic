//! The `pruning` service: free functions named by verb over the steps nothing reads, dropped, and the inputs walked.

use super::types::StepRead;
use crate::domain::rewriting::types::Planned;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::PlanStep;

use ryo_values::domain::plans::service::{each_input, inputs_of, named};

/// The plan without the steps nothing reads, which the idioms leave behind, and the place the
/// emitted step has in what is left. Steps keep their order, and each step's inputs follow.
#[must_use]
pub(crate) fn without_unused(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = without_unused_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// Which steps the roots read, themselves or through the steps they read: each step is marked the first time it is reached, and
/// the inputs of a marked step are followed no more.
#[must_use]
pub(crate) fn read_from(steps: &[PlanStep], roots: &[StepId]) -> Vec<StepRead> {
    let mut wanted = vec![StepRead::Unread; steps.len()];
    let mut pending = roots.to_vec();
    while let Some(id) = pending.pop() {
        let place = usize::try_from(id.0).unwrap_or(usize::MAX);
        #[expect(
            clippy::indexing_slicing,
            reason = "a step reads only steps of its plan; a read past them is a bug that must crash, not a step dropped from the plan"
        )]
        let (seen, step) = (&mut wanted[place], &steps[place]);
        if *seen == StepRead::Read {
            continue;
        }
        *seen = StepRead::Read;
        // Spike (ryo-fmx3): the inputs are read in place, the whole plan is no longer cloned to visit them.
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Step(read) = input {
                pending.push(*read);
            }
        }
    }
    wanted
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn without_unused_roots(steps: Vec<PlanStep>, roots: &mut [StepId]) -> Vec<PlanStep> {
    let wanted = read_from(&steps, roots);
    let mut renumbered = vec![StepId(0); steps.len()];
    let mut next = 0_u32;
    for (slot, keep) in renumbered.iter_mut().zip(&wanted) {
        if *keep == StepRead::Read {
            *slot = StepId(next);
            next += 1;
        }
    }
    let kept = steps
        .into_iter()
        .zip(&wanted)
        .filter(|(_, keep)| **keep == StepRead::Read)
        .map(|(mut step, _)| {
            each_input(&mut step.operation, &mut |input| {
                if let Input::Step(read) = input {
                    *read = kept_as(&renumbered, *read);
                }
            });
            step
        })
        .collect();
    for root in roots.iter_mut() {
        *root = kept_as(&renumbered, *root);
    }
    kept
}

/// The id the step `id` keeps once the steps nothing reads are dropped.
#[expect(
    clippy::indexing_slicing,
    reason = "a kept step reads only steps of its plan, and the emitted step is one; another id is a bug that must crash, not an input left pointing at a dropped step"
)]
fn kept_as(renumbered: &[StepId], id: StepId) -> StepId {
    renumbered[usize::try_from(id.0).unwrap_or(usize::MAX)]
}
