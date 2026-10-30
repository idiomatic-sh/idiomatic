//! The `operations` concept's failures.

/// Why an operation graph does not fold.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OperationsError {
    /// A record of an operation has a count of zero, no reason or a number that is not finite.
    #[error(
        "operation {operation}: record {record} has a count of zero, no reason or a number that is not finite"
    )]
    Invalid {
        /// The operation.
        operation: String,
        /// The record.
        record: String,
    },
    /// An operation decides one record twice.
    #[error("operation {operation} decides record {record} more than once")]
    Twice {
        /// The operation.
        operation: String,
        /// The record.
        record: String,
    },
    /// An operation names a parent the graph does not hold.
    #[error("operation {operation} names the parent {parent}, which the lock does not hold")]
    MissingParent {
        /// The operation.
        operation: String,
        /// The parent.
        parent: String,
    },
    /// The parents form a cycle.
    #[error("the lock's operations form a cycle")]
    Cycle,
}
