//! The types of the `capture_pushing` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::PlanComparison;

/// A filter after a capture and what moves out of it: the capture, what it reads, the comparisons that move before it and those that stay.
pub(crate) struct MovedComparisons {
    /// The place of the capture.
    pub capture: crate::domain::rewriting::types::StepIndex,
    /// What the capture reads.
    pub input: Input,
    /// The comparisons that are made before the capture.
    pub before: Vec<PlanComparison>,
    /// The comparisons that stay where they are.
    pub rest: Vec<PlanComparison>,
}
