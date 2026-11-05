//! The facts port, implemented in memory: a worker keeps the rows of its file in columns it hands out to push onto and read in
//! place, and the words it keeps for the cross-file finish in a column of words for each bucket; the finish joins every worker's
//! column of a bucket into one, in the order of the workers.

use std::collections::BTreeMap;
use std::sync::Mutex;

use ryo_relations::domain::relations::service::{all, columns, kinds};
use ryo_relations::domain::relations::types::Relation;

use super::constants::SCHEMAS;
use super::types::RefusalText;
use ryo_execution::inbound::use_cases::execution::types::facts::FactsError;
use ryo_execution::inbound::use_cases::execution::types::facts::{
    BucketSource, ColumnsMut, KeptBucket, WordsToKeep,
};
use ryo_execution::inbound::use_cases::execution::types::facts::{
    FactsFinish, FactsPort, FactsWorker,
};
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::stored_facts::types::Bucket;

/// The memory facts adapter: it holds nothing, so workers share nothing through it.
#[derive(Debug)]
pub(crate) struct FactsMemoryAdapter;

impl FactsMemoryAdapter {
    /// An adapter whose workers build their columns from each relation of the catalog.
    pub(crate) fn new() -> Self {
        Self
    }
}

/// A refusal, as the port says it.
fn refused(reason: RefusalText<'_>) -> FactsError {
    FactsError::Store {
        reason: reason.0.to_owned(),
    }
}

/// One worker's handle: the columns of each relation for the file it is reading, and the words it keeps in a column for each bucket.
struct MemoryWorker {
    /// The columns of each relation for the worker's file, in the order of all relations.
    tables: Vec<Vec<FactColumn>>,
    /// The words kept so far, in a column for each bucket.
    words: Mutex<BTreeMap<Bucket, KeptBucket>>,
}

impl FactsWorker for MemoryWorker {
    fn resident_stability(
        &self,
    ) -> ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability {
        ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability::Stable
    }

    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        Ok(self
            .tables
            .iter_mut()
            .enumerate()
            .filter_map(|(place, columns)| {
                Some((all().get(place).copied()?, columns.as_mut_slice()))
            })
            .collect())
    }

    fn rows(&self, relation: Relation) -> Result<ColumnsView<'_>, FactsError> {
        let place = all()
            .iter()
            .position(|each| *each == relation)
            .ok_or_else(|| refused(RefusalText(SCHEMAS)))?;
        let columns = self
            .tables
            .get(place)
            .ok_or_else(|| refused(RefusalText(SCHEMAS)))?;
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns,
        })
    }

    fn keep(&self, bucket: Bucket, words: WordsToKeep<'_>) -> Result<(), FactsError> {
        if words.words.0.is_empty() {
            return Err(refused(RefusalText(SCHEMAS)));
        }
        let mut buckets = self
            .words
            .lock()
            .map_err(|_| refused(RefusalText(SCHEMAS)))?;
        let kept = buckets.entry(bucket).or_default();
        let start = kept.words.0.0.len();
        kept.words.0.0.extend_from_slice(&words.words.0);
        kept.sources.push(BucketSource {
            file: words.file,
            start,
            length: words.words.0.len(),
            proof: words.proof.clone(),
        });
        Ok(())
    }

    fn release(&mut self) -> Result<(), FactsError> {
        // Provider columns are a separate owner from the execution pool and file-local indexes.
        let mut remaining = 16_usize * 1024 * 1024;
        for column in self.tables.iter_mut().flatten() {
            column.0.clear();
            let bytes = column.0.capacity().saturating_mul(size_of::<u64>());
            if bytes > remaining {
                column.0 = Vec::new();
            } else {
                remaining -= bytes;
            }
        }
        Ok(())
    }

    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        Ok(Box::new(MemoryFinish {
            kept: self
                .words
                .into_inner()
                .map_err(|_| refused(RefusalText(SCHEMAS)))?,
        }))
    }
}

/// What a worker finished with: its words in each bucket.
struct MemoryFinish {
    /// Its words, in a column for each bucket.
    kept: BTreeMap<Bucket, KeptBucket>,
}

impl FactsFinish for MemoryFinish {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        Ok(self
            .kept
            .get(&bucket)
            .map_or_else(KeptBucket::default, Clone::clone))
    }
}

/// The finished words of every worker, one worker after another: a bucket's words are each worker's in turn.
struct Joined {
    /// What each worker finished with, in the order of the workers.
    parts: Vec<Box<dyn FactsFinish>>,
}

impl FactsFinish for Joined {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        let mut words = KeptBucket::default();
        for part in &self.parts {
            let each = part.kept(bucket)?;
            let offset = words.words.0.0.len();
            words.words.0.0.extend(each.words.0.0);
            words
                .sources
                .extend(each.sources.into_iter().map(|mut source| {
                    source.start += offset;
                    source
                }));
        }
        Ok(words)
    }
}

impl FactsPort for FactsMemoryAdapter {
    fn worker(&self) -> Result<Box<dyn FactsWorker>, FactsError> {
        Ok(Box::new(MemoryWorker {
            tables: all()
                .iter()
                .map(|relation| vec![FactColumn::default(); columns(*relation).len()])
                .collect(),
            words: Mutex::new(BTreeMap::new()),
        }))
    }

    fn finish(
        &self,
        workers: &mut dyn Iterator<Item = Box<dyn FactsWorker>>,
    ) -> Result<Box<dyn FactsFinish>, FactsError> {
        let parts = workers
            .map(FactsWorker::finished)
            .collect::<Result<Vec<_>, FactsError>>()?;
        Ok(Box::new(Joined { parts }))
    }
}
