//! The fake of the `validate_store` port: the versions kept in memory.

use std::sync::Mutex;

use crate::domain::proofs::types::{ProvenVersion, ProvenVersions};
use crate::outbound::validate_store::errors::ValidateStoreError;
use crate::outbound::validate_store::interface::ValidateStorePort;

/// What the core's tests stand in for `validate_store` with.
#[derive(Debug, Default)]
pub struct ValidateStoreFake {
    /// The versions kept.
    pub tables: Mutex<Vec<ProvenVersion>>,
    /// How many times they were kept.
    pub saves: Mutex<usize>,
}

impl ValidateStoreFake {
    /// How many versions the store holds as held.
    #[must_use]
    pub fn held(&self) -> usize {
        self.tables
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }
}

impl ValidateStorePort for ValidateStoreFake {
    fn definitions(&self) -> ProvenVersions {
        ProvenVersions(
            self.tables
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        )
    }

    fn keep_definitions(&self, rows: &[ProvenVersion]) -> Result<(), ValidateStoreError> {
        *self
            .saves
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) += 1;
        rows.clone_into(
            &mut self
                .tables
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
