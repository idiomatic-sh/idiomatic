//! What the `check_store` port asks of the outside, in the core's own vocabulary.

use super::errors::CheckStoreError;
use super::types::Stored;

/// What every `check_store` adapter implements.
pub trait CheckStorePort: std::fmt::Debug + Send + Sync {
    /// What the product's store keeps, read whole; nothing when there is no store, it cannot be read, or another build
    /// wrote it. A check reads it once, at its start.
    fn load(&self) -> Stored;

    /// Keeps `stored` in place of what the store kept, in one atomic write: a check calls it once, at its end, only
    /// when it succeeded, and a write that fails leaves the previous store in place.
    ///
    /// # Errors
    ///
    /// `Unwritable` when the store's directory cannot be written.
    fn keep(&self, stored: &Stored) -> Result<(), CheckStoreError>;
}
