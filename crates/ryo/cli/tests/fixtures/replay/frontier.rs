//! What a per-file plan hands to the cross-file finish, counted: the words each recorded rule's workers keep for it over a corpus of
//! files whose functions hold many tokens. The `lint` bench takes it as a measure (`frontier/<rule>-words`) that asymptote holds to
//! its best; the engine hands a rule the words of a row for each function, and a rule that hands words in proportion to the tokens
//! of a file is the blowup the ratchet stops from rising.

use crate::base;
use crate::counting;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::{FactsFinish, FactsPort, FactsWorker};
use ryo_execution_runtime::outbound::facts::types::{ColumnsMut, WordsToKeep};
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::stored_facts::types::Bucket;
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The adapter, counting the words the files leave for the finish.
pub(crate) struct Counting {
    /// The store it counts for.
    pub store: Arc<dyn FactsPort>,
    /// The words its workers kept.
    pub words: Arc<AtomicUsize>,
}

/// A handle of the adapter, counting the words its worker keeps.
pub(crate) struct CountingWorker {
    inner: Box<dyn FactsWorker>,
    words: Arc<AtomicUsize>,
}

impl FactsWorker for CountingWorker {
    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        self.inner.columns_mut()
    }
    fn rows(&self, relation: Relation) -> Result<ColumnsView<'_>, FactsError> {
        self.inner.rows(relation)
    }
    fn keep(&self, bucket: Bucket, words: WordsToKeep<'_>) -> Result<(), FactsError> {
        // A packed word ends at the byte whose high bit is clear: the count is of words, not of bytes.
        let count = words
            .words
            .0
            .iter()
            .filter(|byte| *byte & 0x80 == 0)
            .count();
        let _before = self.words.fetch_add(count, Ordering::Relaxed);
        self.inner.keep(bucket, words)
    }
    fn release(&mut self) -> Result<(), FactsError> {
        self.inner.release()
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        self.inner.finished()
    }
}

impl FactsPort for Counting {
    fn worker(&self) -> Result<Box<dyn FactsWorker>, FactsError> {
        Ok(Box::new(CountingWorker {
            inner: self.store.worker()?,
            words: Arc::clone(&self.words),
        }))
    }
    fn finish(
        &self,
        workers: &mut dyn Iterator<Item = Box<dyn FactsWorker>>,
    ) -> Result<Box<dyn FactsFinish>, FactsError> {
        self.store.finish(workers)
    }
}

/// Files of `functions` functions each, each holding `statements` repeats of one statement of about eight tokens: more elements, the same
/// vocabulary, so what a rule hands over may depend on the first and never on the second.
pub(crate) fn corpus(files: usize, functions: usize, statements: usize) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a directory");
    for file in 0..files {
        let mut text = String::new();
        for function in 0..functions {
            writeln!(text, "fn work_{function}() {{").expect("writing to a String cannot fail");
            for _ in 0..statements {
                writeln!(text, "    let value = {file} + 1;")
                    .expect("writing to a String cannot fail");
            }
            text.push_str("}\n");
        }
        std::fs::write(root.path().join(format!("file_{file}.rs")), text).expect("a file");
    }
    root
}

/// The words the finish is handed by one recorded rule for the corpus.
pub(crate) fn handed_by(root: &std::path::Path, name: &str) -> usize {
    let (_, rule) = counting::runnable()
        .into_iter()
        .find(|(each, _)| each == name)
        .expect("the recorded rule");
    let counting = Arc::new(Counting {
        store: Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
        words: Arc::new(AtomicUsize::new(0)),
    });
    let _rows = base::lint_with(
        root,
        vec![rule],
        Arc::<Counting>::clone(&counting),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("the rule runs");
    counting.words.load(Ordering::Relaxed)
}
