//! The `catalog` port's failures.

/// Why the `catalog` port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CatalogError {
    /// The catalog could not be read.
    #[error("the relation catalog is unavailable: {reason}")]
    Unavailable {
        /// What went wrong.
        reason: String,
    },
}
