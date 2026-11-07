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

    fn plans(&self) -> Vec<ryo_execution::inbound::use_cases::prepare::interface::CachedPlan> {
        Vec::new()
    }

    fn keep_plans(
        &self,
        _plans: &[ryo_execution::inbound::use_cases::prepare::interface::CachedPlan],
    ) -> Result<(), LintStoreError> {
        Ok(())
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

    fn read_old(
        &self,
        _words: &ryo_values::domain::stores::types::OldWords,
        _into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError> {
        Err(LintStoreError::NothingRead)
    }

    fn begin_old_rows(
        &self,
        _wanted: &ryo_values::domain::stores::types::BucketsWanted,
    ) -> Result<Option<Box<dyn crate::outbound::lint_store::interface::OldRowBatch>>, LintStoreError> {
        Ok(None)
    }

    fn stage_words(
        &self,
        words: ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<ryo_values::domain::records::types::RecordWords, LintStoreError> {
        Ok(ryo_values::domain::records::types::RecordWords::Inline(words))
    }

    fn read_staged(
        &self,
        _words: &ryo_values::domain::records::types::StagedWords,
        _into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError> {
        Err(LintStoreError::NothingRead)
    }

    fn keep(&self, _stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError> {
        if !kept.staged.is_empty() {
            return Err(LintStoreError::NothingRead);
        }
        Ok(())
    }
}
