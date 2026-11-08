//! The facts port, implemented in memory: a worker keeps the rows of its file in columns it hands out to push onto and read in
//! place, and the words it keeps for the cross-file finish in a column of words for each bucket; the finish joins every worker's
//! column of a bucket into one, in the order of the workers.

use std::collections::BTreeMap;
use std::sync::Mutex;

use ryo_relations::domain::relations::service::{all, columns, kinds, number_of};
use ryo_relations::domain::relations::types::Relation;

use super::constants::{RETAINED_COLUMN_WORDS, SCHEMAS};
use super::types::FactsRefusalText;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::types::{BucketSource, ColumnsMut, KeptBucket, WordsToKeep};
use crate::outbound::facts::interface::{FactsFinish, FactsPort, FactsWorker};
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
fn refused(reason: FactsRefusalText<'_>) -> FactsError {
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
    ) -> crate::outbound::facts::types::ResidentStability {
        crate::outbound::facts::types::ResidentStability::Stable
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
        let columns = self
            .tables
            .get(number_of(relation).0)
            .ok_or_else(|| refused(FactsRefusalText(SCHEMAS)))?;
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns,
        })
    }

    fn keep(&self, bucket: Bucket, words: WordsToKeep<'_>) -> Result<(), FactsError> {
        if words.words.0.is_empty() {
            return Err(refused(FactsRefusalText(SCHEMAS)));
        }
        let mut buckets = self
            .words
            .lock()
            .map_err(|_| refused(FactsRefusalText(SCHEMAS)))?;
        let kept = buckets.entry(bucket).or_default();
        // The worker is the only holder of its words until it finishes: this borrows them mutably and copies nothing.
        let held = &mut std::sync::Arc::make_mut(&mut kept.words).0.0;
        let start = held.len();
        held.extend_from_slice(&words.words.0);
        kept.sources.push(BucketSource {
            file: words.file,
            start: crate::outbound::facts::types::KeptWordsStart(start),
            length: crate::outbound::facts::types::KeptWordsLength(words.words.0.len()),
            proof: words.proof.clone(),
        });
        Ok(())
    }

    fn release(&mut self) -> Result<(), FactsError> {
        // A column keeps room for a small file's rows and gives the rest back: the next file grows it again when it needs more.
        for column in self.tables.iter_mut().flatten() {
            column.0.clear();
            column.0.shrink_to(RETAINED_COLUMN_WORDS);
        }
        Ok(())
    }

    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        Ok(Box::new(MemoryFinish {
            kept: self
                .words
                .into_inner()
                .map_err(|_| refused(FactsRefusalText(SCHEMAS)))?,
        }))
    }
}

/// What a worker finished with: its words in each bucket.
struct MemoryFinish {
    /// Its words, in a column for each bucket.
    kept: BTreeMap<Bucket, KeptBucket>,
}

impl FactsFinish for MemoryFinish {
    /// The worker's own buffer of the bucket, shared: its words are not copied.
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
    /// One worker's words are handed out as that worker holds them; the words of several are run together once, into a buffer
    /// of exactly their length.
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        let mut held = Vec::with_capacity(self.parts.len());
        for part in &self.parts {
            let each = part.kept(bucket)?;
            if !each.sources.is_empty() || !each.words.0.0.is_empty() {
                held.push(each);
            }
        }
        if held.len() <= 1 {
            return Ok(held.pop().unwrap_or_default());
        }
        let length = held.iter().map(|each| each.words.0.0.len()).sum();
        let mut words = Vec::with_capacity(length);
        let mut sources = Vec::with_capacity(held.iter().map(|each| each.sources.len()).sum());
        for each in held {
            let offset = words.len();
            words.extend_from_slice(&each.words.0.0);
            sources.extend(each.sources.into_iter().map(|mut source| {
                source.start.0 += offset;
                source
            }));
        }
        Ok(KeptBucket {
            words: std::sync::Arc::new(ryo_values::domain::stored_facts::types::KeptWords(
                ryo_values::domain::stored_facts::types::PackedWords(words),
            )),
            sources,
        })
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
