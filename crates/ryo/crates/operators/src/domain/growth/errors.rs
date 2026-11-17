//! The refusal of the `growth` concept's plan check.

/// Why the plan check refused a plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum GrowthError {
    /// A node's rows grow faster than the rows of every input it reads, and the query did not declare every pair the answer.
    #[error(
        "node {node:?} ({growth:?}) pairs every row of a file with every row of it (grows as rows^{degree:?}): write \
         `nearest join` to keep the closest row, or `all-pairs join` to declare every pair"
    )]
    Superlinear {
        /// The node that causes the growth.
        node: crate::domain::physical_plan::types::PlanNodeId,
        /// Its growth.
        growth: super::types::Growth,
        /// The degree in rows of its output or its work.
        degree: super::types::Degree,
    },
}
