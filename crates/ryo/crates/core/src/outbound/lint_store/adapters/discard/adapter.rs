//! The store port, implemented as no store: what a run gives it is let go, and it reads back as empty, so a run over it makes
//! nothing only a store reads.

use std::sync::Arc;

use super::types::DiscardRead;
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::interface::{LintStorePort, ReadStore};
use crate::outbound::lint_store::types::StoreKeeping;
use ryo_values::domain::stores::types::{BucketsWanted, KeptRows, Stored};

/// The discard store adapter.
#[derive(Debug, Default)]
pub(crate) struct LintStoreDiscardAdapter;

impl LintStorePort for LintStoreDiscardAdapter {
    fn keeps(&self) -> StoreKeeping {
        StoreKeeping::Discards
    }

    fn stored(&self) -> Result<Arc<dyn ReadStore>, LintStoreError> {
        Ok(Arc::new(DiscardRead))
    }
}

impl ReadStore for DiscardRead {
    fn tables(&self) -> Stored {
        Stored::default()
    }

    fn handed(
        &self,
        _wanted: &BucketsWanted,
    ) -> Result<ryo_values::domain::stores::types::ReadHanded, LintStoreError> {
        Ok(ryo_values::domain::stores::types::ReadHanded::default())
    }

    fn keep(&self, _stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError> {
        if !kept.staged.is_empty() {
            return Err(LintStoreError::NothingRead);
        }
        Ok(())
    }
}
