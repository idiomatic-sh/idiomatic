//! The store port, implemented in memory: what a run keeps lives as long as the adapter, so a process that lints once keeps nothing.

use std::sync::{Arc, PoisonError};

use super::types::{KeptStore, MemoryRead};
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::interface::{LintStorePort, ReadStore};
use crate::outbound::lint_store::types::StoreKeeping;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{
    BucketsWanted, KeptRows, Stored, StoredHanded, StoredWords,
};

/// The memory store adapter.
#[derive(Debug, Default)]
pub(crate) struct LintStoreMemoryAdapter {
    /// What the last run kept; a run that read it holds it as it was whatever a run keeps after.
    kept: KeptStore,
}

impl LintStorePort for LintStoreMemoryAdapter {
    fn keeps(&self) -> StoreKeeping {
        StoreKeeping::Keeps
    }

    fn stored(&self) -> Result<Arc<dyn ReadStore>, LintStoreError> {
        let was = Arc::clone(&self.kept.0.lock().unwrap_or_else(PoisonError::into_inner));
        Ok(Arc::new(MemoryRead {
            cell: self.kept.clone(),
            was,
        }))
    }
}

impl ReadStore for MemoryRead {
    fn tables(&self) -> Stored {
        Stored {
            handed: StoredHanded::default(),
            ..(*self.was).clone()
        }
    }

    fn handed(
        &self,
        wanted: &BucketsWanted,
    ) -> Result<ryo_values::domain::stores::types::ReadHanded, LintStoreError> {
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

    fn keep(&self, mut stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError> {
        if !kept.staged.is_empty() {
            return Err(LintStoreError::NothingRead);
        }
        if *self.was == Stored::default()
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
        *self.cell.0.lock().unwrap_or_else(PoisonError::into_inner) = Arc::new(stored);
        Ok(())
    }
}
