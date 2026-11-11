//! The cases of captured frontier buckets and the staging of fresh outcomes.
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::interface::ReadStore;
use ryo_values::domain::records::types::RecordWords;
use ryo_values::domain::records::types::StagedWords;
use ryo_values::domain::records::types::StagingSession;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stores::types::BucketsWanted;
use ryo_values::domain::stores::types::KeptRows;
use ryo_values::domain::stores::types::Stored;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[derive(Debug, Default)]
pub(crate) struct StagingRead {
    pub(crate) calls: AtomicUsize,
    pub(crate) fail: bool,
}

impl ReadStore for StagingRead {
    fn tables(&self) -> Stored {
        Stored::default()
    }
    fn handed(
        &self,
        _: &BucketsWanted,
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
    ) -> Result<Option<Box<dyn crate::outbound::lint_store::interface::OldRowBatch>>, LintStoreError>
    {
        Ok(None)
    }

    fn read_staged(
        &self,
        _words: &ryo_values::domain::records::types::StagedWords,
        _into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError> {
        Err(LintStoreError::NothingRead)
    }

    fn keep(&self, _: Stored, _: &KeptRows) -> Result<(), LintStoreError> {
        Ok(())
    }
    fn stage_words(&self, words: PackedWords) -> Result<RecordWords, LintStoreError> {
        let offset = self.calls.fetch_add(1, Ordering::Relaxed);
        if self.fail {
            return Err(LintStoreError::NothingRead);
        }
        Ok(RecordWords::Staged(StagedWords {
            session: StagingSession(1),
            offset: ryo_values::domain::records::types::PayloadOffset(
                u64::try_from(offset).expect("small count"),
            ),
            length: ryo_values::domain::records::types::PayloadLength(
                u64::try_from(words.0.len()).expect("size"),
            ),
        }))
    }
}
