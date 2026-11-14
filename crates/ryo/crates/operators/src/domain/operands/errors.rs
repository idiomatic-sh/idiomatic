//! The errors of the `operands` concept.

/// Why one side of a comparison could not be read for a row.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OperandsError {
    /// A parameter the operand reads has no binding.
    #[error("unbound parameter")]
    Unbound,
    /// A bound value does not fit the operand.
    #[error("wrong binding")]
    WrongBinding,
    /// A cell the operand reads is not in the row.
    #[error(transparent)]
    Rows(#[from] ryo_values::domain::rows::errors::RowsError),
}
