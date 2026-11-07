//! The fake of the `lint_store` port: what the core's tests stand in for it with, and the tables they keep in it.

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use super::super::errors::LintStoreError;
use super::super::interface::LintStorePort;
use super::super::interface::ReadStore;
use super::super::types::StoreKeeping;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::BucketsWanted;
use ryo_values::domain::stores::types::KeptRows;
use ryo_values::domain::stores::types::Stored;
use ryo_values::domain::stores::types::StoredHanded;
use ryo_values::domain::stores::types::StoredWords;

/// What the core's tests stand in for `lint_store` with: the tables kept in memory, and how often each call was made.
#[derive(Debug, Default)]
pub struct LintStoreFake {
    /// What the last run kept.
    pub kept: Arc<Mutex<Stored>>,
    /// Whether every read fails.
    pub unreadable: bool,
    /// Whether every keep fails.
    pub unwritable: bool,
    /// How many times the tables were read.
    pub reads: Arc<AtomicUsize>,
    /// How many times the tables were kept.
    pub keeps: Arc<AtomicUsize>,
}

/// The fake's stream of old rows: it holds none, so every row it is asked for is refused as the fake's `read_old` refuses it.
#[derive(Debug, Default)]
pub struct OldRowBatchFake;

impl crate::outbound::lint_store::interface::OldRowBatch for OldRowBatchFake {
    fn read(
        &mut self,
        _words: &ryo_values::domain::stores::types::OldWords,
        _into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError> {
        Err(LintStoreError::NothingRead)
    }
}

/// The fake's store as one run read it: what it held then, and the places the next keep and the counts go to.
#[derive(Debug)]
pub struct FakeRead {
    /// The fake the run read.
    fake: LintStoreFake,
    /// What it held when the run read it.
    was: Stored,
}

impl LintStorePort for LintStoreFake {
    fn keeps(&self) -> StoreKeeping {
        StoreKeeping::Keeps
    }

    fn stored(&self) -> Result<Arc<dyn ReadStore>, LintStoreError> {
        let _reads_before = self.reads.fetch_add(1, Ordering::Relaxed);
        if self.unreadable {
            return Err(LintStoreError::Unreadable {
                directory: String::new(),
                reason: String::new(),
            });
        }
        let was = self
            .kept
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        Ok(Arc::new(FakeRead {
            fake: LintStoreFake {
                kept: Arc::clone(&self.kept),
                unreadable: self.unreadable,
                unwritable: self.unwritable,
                reads: Arc::clone(&self.reads),
                keeps: Arc::clone(&self.keeps),
            },
            was,
        }))
    }
}

impl ReadStore for FakeRead {
    fn tables(&self) -> Stored {
        Stored {
            handed: StoredHanded::default(),
            ..self.was.clone()
        }
    }

    fn handed(
        &self,
        wanted: &BucketsWanted,
    ) -> Result<ryo_values::domain::stores::types::ReadHanded, LintStoreError> {
        let _reads_before = self.fake.reads.fetch_add(1, Ordering::Relaxed);
        if self.fake.unreadable {
            return Err(LintStoreError::Unreadable {
                directory: String::new(),
                reason: String::new(),
            });
        }
        let mut handed = self.was.handed.clone();
        if let BucketsWanted::These(buckets) = wanted {
            handed
                .frontier
                .retain(|each| buckets.binary_search(&each.bucket).is_ok());
            handed
                .proofs
                .retain(|each| buckets.binary_search(&each.bucket).is_ok());
        }
        Ok(ryo_values::domain::stores::service::read_handed(handed))
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

    fn keep(&self, mut stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError> {
        if !kept.staged.is_empty() {
            return Err(LintStoreError::NothingRead);
        }
        let _keeps_before = self.fake.keeps.fetch_add(1, Ordering::Relaxed);
        if self.fake.unwritable {
            return Err(LintStoreError::Unwritable {
                directory: String::new(),
                reason: String::new(),
            });
        }
        if self.was == Stored::default()
            && !(kept.unchanged.is_empty() && kept.file_again.is_empty())
        {
            return Err(LintStoreError::NothingRead);
        }
        // The words of the buckets as they were stay as they were kept, unless the run made them again; in every other bucket the
        // words of a file used again stay, under its number now.
        let made: Vec<Bucket> = stored
            .handed
            .frontier
            .iter()
            .map(|each| each.bucket)
            .collect();
        stored
            .handed
            .frontier
            .extend(self.was.handed.frontier.iter().filter_map(|each| {
                if kept.unchanged.binary_search(&each.bucket).is_ok() {
                    return (!made.contains(&each.bucket)).then(|| each.clone());
                }
                // A file not used again is gone, with its words.
                match kept
                    .file_again
                    .binary_search_by_key(&each.file, |again| again.was)
                {
                    Ok(place) => kept.file_again.get(place).map(|again| StoredWords {
                        file: again.now,
                        ..each.clone()
                    }),
                    Err(_) => None,
                }
            }));
        stored
            .handed
            .frontier
            .sort_by_key(|each| (each.file, each.bucket));
        ryo_values::domain::stores::service::carry_proofs(
            &mut stored.handed,
            &self.was.handed,
            kept,
        );
        *self
            .fake
            .kept
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = stored;
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
