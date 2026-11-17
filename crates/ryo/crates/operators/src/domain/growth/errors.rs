//! The refusals of the `growth` concept: the plan check's and the run budget's.
use ryo_values::domain::rows::types::RowCount;

/// Why a node's rows broke the growth they are held to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum GrowthError {
    /// A node's rows grow faster than the rows of every input it reads, and the query did not declare every pair the answer.
    #[cfg(test)]
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
    /// A unit's makings of a node wrote past the run budget: more rows than the budget allows for the rows they were fed, or,
    /// for an operator declared to write at most its input, more rows than they were fed. It names no node and no file: its
    /// caller knows which it made.
    #[error("wrote {} rows from {} rows fed, past the run budget", wrote.0, fed.0)]
    OverBudget {
        /// The rows they were fed, every input summed.
        fed: RowCount,
        /// The rows they wrote.
        wrote: RowCount,
    },
}
