//! The errors of the `growth` concept.
use ryo_values::domain::rows::types::RowCount;

/// Why a node's rows broke the growth they are held to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum GrowthError {
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
