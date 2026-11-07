//! The `pruning` service: free functions named by verb over the steps nothing reads, dropped, and the inputs walked.

use crate::domain::rewriting::types::Planned;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::PlanStep;

use ryo_values::domain::plans::service::each_input;

/// The plan without the steps nothing reads, which the idioms leave behind, and the place the
/// emitted step has in what is left. Steps keep their order, and each step's inputs follow.
#[must_use]
pub(crate) fn without_unused(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = without_unused_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn without_unused_roots(steps: Vec<PlanStep>, roots: &mut [StepId]) -> Vec<PlanStep> {
    let mut wanted = vec![false; steps.len()];
    let mut pending = roots.to_vec();
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
