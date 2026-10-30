//! What the `validate_store` port asks of the outside, in the core's own vocabulary: the versions `validate` proved
//! held, kept between runs.

use super::errors::ValidateStoreError;
use crate::domain::proofs::types::{ProvenVersion, ProvenVersions};

/// What every `validate_store` adapter implements.
pub trait ValidateStorePort: Send + Sync {
    /// The versions kept as held, read whole; none when the store is another build's, keeps none, or does not read.
    fn definitions(&self) -> ProvenVersions;

    /// Keeps `rows` as the versions that held, replacing the store whole: a reader sees the old store or the new, never
    /// part of one.
    ///
    /// # Errors
    ///
    /// `Unwritable` when the store cannot be written.
    fn keep_definitions(&self, rows: &[ProvenVersion]) -> Result<(), ValidateStoreError>;
}
