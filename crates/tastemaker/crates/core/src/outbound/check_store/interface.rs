//! What the `check_store` port asks of the outside, in the core's own vocabulary: the answer `check` gave and the files
//! it rests on, kept between runs.

use super::errors::CheckStoreError;
use super::types::Checked;

/// What every `check_store` adapter implements.
pub trait CheckStorePort: Send + Sync {
    /// What the last check kept, read whole; nothing when the store is another build's, keeps none, or does not read.
    fn checked(&self) -> Checked;

    /// Keeps `checked` as the check's answer, replacing the store whole: a reader sees the old store or the new, never
    /// part of one.
    ///
    /// # Errors
    ///
    /// `Unwritable` when the store cannot be written.
    fn keep_checked(&self, checked: &Checked) -> Result<(), CheckStoreError>;
}
