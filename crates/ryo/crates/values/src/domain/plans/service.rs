//! The `plans` service: free functions named by verb over a compiled plan, with no IO and no ports.

use super::types::{Operation, Plan, StepScope};
use crate::domain::plan_parts::service::step_in;
use crate::domain::plan_parts::types::Input;
use crate::domain::plan_parts::types::StepId;
use crate::domain::plan_parts::types::{InputCount, Inputs};

/// What an operation reads, in the order its operator reads its inputs.
#[must_use]
pub fn inputs_of(operation: &Operation) -> Inputs {
    let (first, second, third, used) = match operation {
        Operation::Replace { from, .. }
        | Operation::Select { from, .. }
        | Operation::Project { from, .. }
        | Operation::Extend { from, .. }
        | Operation::GroupCount { from, .. }
        | Operation::GroupDigest { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Rank { from, .. }
        | Operation::Words { from, .. } => (*from, *from, *from, 1),
        Operation::OrderedMatch { left, right, .. }
        | Operation::Join { left, right, .. }
        | Operation::Union { left, right }
        | Operation::Intersection { left, right }
        | Operation::Difference { left, right }
        | Operation::Antijoin { left, right, .. } => (*left, *right, *left, 2),
        Operation::Axis {
            from,
            edges,
            domain,
            ..
        } => (*from, *edges, *domain, 3),
        Operation::Closure { edges, .. } | Operation::Cycles { edges, .. } => {
            (*edges, *edges, *edges, 1)
        }
        // The host's rows read nothing: the place filled in is never asked for.
        Operation::Rows { .. } | Operation::Hosted { .. } => {
            let nothing = Input::Step(StepId(0));
            (nothing, nothing, nothing, 0)
        }
    };
    Inputs {
        list: [first, second, third],
        used: InputCount(used),
    }
}

/// The inputs an operation names, without the places that repeat.
#[must_use]
pub fn named(inputs: &Inputs) -> &[Input] {
    inputs.list.get(..inputs.used.0).unwrap_or(&inputs.list)
}

/// Where the step `id` runs, when the plan has it.
#[must_use]
pub fn step_scope(plan: &Plan, id: StepId) -> Option<StepScope> {
    step_in(&plan.steps, id).map(|step| step.scope)
}
