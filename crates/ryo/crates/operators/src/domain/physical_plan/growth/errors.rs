//! The refusal of the plan's growth check.

/// Why the plan check refused a plan. Its text is the contract's `RefusedNode`, which the plan makes of it with the rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GrowthError {
    /// A node's rows grow faster than the rows of every input it reads, and the query did not declare every pair the answer.
    Superlinear {
        /// The node that causes the growth.
        node: crate::domain::physical_plan::types::PlanNodeId,
        /// The name of its kind.
        kind: &'static str,
        /// The degree in rows of its output or its work.
        degree: crate::domain::growth::types::Degree,
    },
}
