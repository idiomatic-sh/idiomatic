//! The fake of the `facts` port: what the core's tests stand in for it with. It keeps its numbers in memory, and fails when told to.

use ryo_relations::domain::relations::collections::relations::function::item::{
    Function, FunctionRow,
};
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem as _;
use ryo_relations::domain::relations::types::{FileId, NodeId, Offset, Span};
use ryo_relations::domain::terms::types::Symbol;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ryo_relations::domain::relations::service::{all, columns, kinds};
use ryo_relations::domain::relations::types::Relation;

use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::{FactsFinish, FactsPort, FactsWorker};
use crate::outbound::facts::types::{BucketSource, ColumnsMut, KeptBucket, WordsToKeep};
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::stored_facts::types::Bucket;

/// What the core's tests stand in for `facts` with.
#[derive(Debug, Default)]
pub struct FactsFake {
    /// Whether every call fails.
    pub fails: bool,
    /// How many times the port, or a handle it opened, was called.
    pub calls: Arc<AtomicUsize>,
}

/// What the fake answers when it refuses.
fn refusal() -> FactsError {
    FactsError::Store {
        reason: String::new(),
    }
}

impl FactsFake {
    /// A fake whose every call fails.
    #[must_use]
    pub fn failing() -> Self {
        Self {
            fails: true,
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// How many times the port, or a handle it opened, was called so far.
    #[must_use]
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }

    /// A worker of this fake, handed to a test that names only the worker, never the port.
    ///
    /// # Errors
    ///
    /// `Store` when the fake is told to fail.
    pub fn open_worker(&self) -> Result<Box<dyn FactsWorker>, FactsError> {
        FactsPort::worker(self)
    }
}

/// Counts a call, and refuses it when told to fail.
fn counted(calls: &AtomicUsize, fails: bool) -> Result<(), FactsError> {
    let _calls_before = calls.fetch_add(1, Ordering::Relaxed);
    if fails { Err(refusal()) } else { Ok(()) }
}

/// A handle of the fake: the columns of each relation, and the words kept in each bucket.
struct FakeWorker {
    /// Whether every call fails, as the fake that opened the handle does.
    fails: bool,
    /// The fake's count of calls, which the handle's calls add to.
    calls: Arc<AtomicUsize>,
    /// The columns of each relation, in the order of all relations.
    tables: Vec<Vec<FactColumn>>,
    /// The words kept in each bucket.
    words: Mutex<BTreeMap<Bucket, KeptBucket>>,
}

impl FactsWorker for FakeWorker {
    fn resident_stability(&self) -> crate::outbound::facts::types::ResidentStability {
        crate::outbound::facts::types::ResidentStability::Stable
    }

    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        counted(&self.calls, self.fails)?;
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
        counted(&self.calls, self.fails)?;
        let place = all()
            .iter()
            .position(|each| *each == relation)
            .ok_or_else(refusal)?;
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns: self.tables.get(place).ok_or_else(refusal)?,
        })
    }

    fn keep(&self, bucket: Bucket, words: WordsToKeep<'_>) -> Result<(), FactsError> {
        counted(&self.calls, self.fails)?;
        if words.words.0.is_empty() {
            return Err(refusal());
        }
        let mut buckets = self.words.lock().map_err(|_| refusal())?;
        let kept = buckets.entry(bucket).or_default();
        let held = &mut Arc::make_mut(&mut kept.words).0.0;
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
        counted(&self.calls, self.fails)?;
        for column in self.tables.iter_mut().flatten() {
            column.0.clear();
        }
        Ok(())
    }

    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        counted(&self.calls, self.fails)?;
        Ok(Box::new(FakeFinish {
            kept: self.words.into_inner().map_err(|_| refusal())?,
        }))
    }
}

/// What a fake worker finished with: its words in each bucket.
struct FakeFinish {
    /// Its words, in each bucket.
    kept: BTreeMap<Bucket, KeptBucket>,
}

impl FactsFinish for FakeFinish {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        Ok(self.kept.get(&bucket).cloned().unwrap_or_default())
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
            let held = &mut Arc::make_mut(&mut words.words).0.0;
            let offset = held.len();
            held.extend_from_slice(&each.words.0.0);
            words
                .sources
                .extend(each.sources.into_iter().map(|mut source| {
                    source.start.0 += offset;
                    source
                }));
        }
        Ok(words)
    }
}

impl FactsPort for FactsFake {
    fn worker(&self) -> Result<Box<dyn FactsWorker>, FactsError> {
        counted(&self.calls, self.fails)?;
        Ok(Box::new(FakeWorker {
            fails: self.fails,
            calls: Arc::clone(&self.calls),
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
        counted(&self.calls, self.fails)?;
        let parts = workers
            .map(FactsWorker::finished)
            .collect::<Result<Vec<_>, FactsError>>()?;
        Ok(Box::new(Joined { parts }))
    }
}

/// The words of one row of `function`, in the file `file`.
pub(super) fn one_function(file: FileId) -> [u64; 4] {
    let words = Function::words(FunctionRow {
        id: NodeId(1),
        name: Symbol(0),
        file,
        span: Span {
            start: Offset(0),
            end: Offset(4),
        },
    });
    words.map(|word| word.0)
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
