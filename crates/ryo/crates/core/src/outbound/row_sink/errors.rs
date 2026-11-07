//! The `row_sink` port's failures.

/// Why the `row_sink` port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RowSinkError {
    /// Whoever reads the rows stopped reading: nothing more is wanted.
    #[error("whoever reads the rows stopped reading")]
    Closed,
    /// The rows could not be written.
    #[error("cannot write the rows: {reason}")]
    Failed {
        /// What went wrong.
        reason: String,
    },
}
