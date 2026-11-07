//! The `receipts` port's failures.

/// Why an operation of `receipts` failed.
#[derive(Debug, thiserror::Error)]
pub enum ReceiptsError {
    /// Receipt IO failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The receipt name, key or row was refused.
    #[error("{0}")]
    Invalid(String),
}
