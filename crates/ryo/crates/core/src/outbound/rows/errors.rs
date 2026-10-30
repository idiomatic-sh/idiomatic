//! The `rows` port's failures.

/// Why the `rows` port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RowsError {
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
