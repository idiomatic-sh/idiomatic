//! The store port, implemented as no store: what a run gives it is let go, and it reads back as empty, so a run over it makes
//! nothing only a store reads.

use std::sync::Arc;

use super::types::DiscardRead;
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::interface::{LintStorePort, ReadStore};
use crate::outbound::lint_store::types::StoreKeeping;
use ryo_values::domain::stores::types::{BucketsWanted, KeptRows, Stored, StoredHanded};

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

    fn handed(&self, _wanted: &BucketsWanted) -> Result<StoredHanded, LintStoreError> {
        Ok(StoredHanded::default())
    }

    fn keep(&self, _stored: Stored, _kept: &KeptRows) -> Result<(), LintStoreError> {
        Ok(())
    }
}
