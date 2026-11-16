//! The stores a replay test runs over: the fact store counting the words its workers keep for the finish, and the store of what a
//! run keeps counting what each keep is given.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ryo_core::outbound::lint_store::errors::LintStoreError;
use ryo_core::outbound::lint_store::interface::{LintStorePort, ReadStore};
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_execution_runtime::outbound::facts::types::ColumnsMut;
use ryo_execution_runtime::outbound::facts::types::WordsToKeep;
use ryo_relations::domain::relations::types::ColumnsView;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{BucketsWanted, KeptRows, Stored};

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

/// The complete emitted output and report of a run, which the cache must give again unchanged.
pub(crate) struct Complete {
    /// The emitted columns, ordered cells, unread reasons and summary.
    pub output: ryo_core::outbound::row_sink::types::Collected,
    /// The successful report, with its unknowns and work counters.
    pub report: ryo_core::inbound::use_cases::lint::types::LintReport,
}

/// A run of `rules` over `root` keeping what it found in `kept`: what it counted, and its complete output and report.
pub(crate) fn complete(
    root: &Path,
    rules: Vec<ryo_core::inbound::use_cases::lint::types::Rule>,
    kept: &Arc<dyn LintStorePort>,
    run: &str,
) -> (super::counting::Counted, Complete) {
    super::counting::lint_observing(
        root,
        rules,
        Arc::clone(kept),
        0,
        None,
        |counted, output, report| (counted, Complete { output, report }),
    )
    .expect(run)
}

/// The store of what a run keeps, counting what the last keep was given: the files whose words it was handed, and the files used
/// again whose words it keeps where they are.
pub(crate) struct CountingStore {
    /// The store it counts for.
    pub store: Arc<dyn LintStorePort>,
    /// How many files the last keep was handed words of.
    pub handed_files: Arc<AtomicUsize>,
    /// How many files used again the last keep moved the words of.
    pub files_again: Arc<AtomicUsize>,
}

impl LintStorePort for CountingStore {
    fn keeps(&self) -> ryo_core::outbound::lint_store::types::StoreKeeping {
        self.store.keeps()
    }
    fn stored(&self) -> Result<Arc<dyn ReadStore>, LintStoreError> {
        Ok(Arc::new(CountingRead {
            read: self.store.stored()?,
            handed_files: Arc::clone(&self.handed_files),
            files_again: Arc::clone(&self.files_again),
        }))
    }
}

/// A store a run read through the counting store: it counts what the run keeps.
#[derive(Debug)]
struct CountingRead {
    /// The store it counts for, as the run read it.
    read: Arc<dyn ReadStore>,
    /// How many files the last keep was handed words of.
    handed_files: Arc<AtomicUsize>,
    /// How many files used again the last keep moved the words of.
    files_again: Arc<AtomicUsize>,
}

impl ReadStore for CountingRead {
    fn tables(&self) -> Stored {
        self.read.tables()
    }
    fn handed(
        &self,
        wanted: &BucketsWanted,
    ) -> Result<ryo_values::domain::stores::types::ReadHanded, LintStoreError> {
        self.read.handed(wanted)
    }
    fn stage_words(
        &self,
        words: ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<ryo_values::domain::records::types::RecordWords, LintStoreError> {
        self.read.stage_words(words)
    }
    fn read_old(
        &self,
        words: &ryo_values::domain::stores::types::OldWords,
        into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError> {
        self.read.read_old(words, into)
    }
    fn read_staged(
        &self,
        words: &ryo_values::domain::records::types::StagedWords,
        into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError> {
        self.read.read_staged(words, into)
    }
    fn begin_old_rows(
        &self,
        _wanted: &BucketsWanted,
    ) -> Result<
        Option<Box<dyn ryo_core::outbound::lint_store::interface::OldRowBatch>>,
        LintStoreError,
    > {
        Ok(None)
    }
    fn keep(&self, stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError> {
        let mut files: Vec<u32> = stored
            .handed
            .frontier
            .iter()
            .map(|each| each.file.0)
            .chain(kept.staged.iter().map(|each| each.file.0))
            .collect();
        files.sort_unstable();
        files.dedup();
        self.handed_files.store(files.len(), Ordering::Relaxed);
        self.files_again
            .store(kept.file_again.len(), Ordering::Relaxed);
        self.read.keep(stored, kept)
    }
}

/// A store that keeps nothing, as the command line runs without a cache.
pub(crate) fn nothing_kept() -> Arc<dyn LintStorePort> {
    Arc::new(ryo_core::outbound::lint_store::factory::discard())
}

/// The store of what a run keeps under `root`, counting what each keep is given.
pub(crate) fn counting_at(root: &Path) -> Arc<CountingStore> {
    Arc::new(CountingStore {
        store: super::base::store_at(root),
        handed_files: Arc::new(AtomicUsize::new(0)),
        files_again: Arc::new(AtomicUsize::new(0)),
    })
}

/// A store as the replay holds one: shared between the runs that keep into it.
#[derive(Clone)]
pub(crate) struct Kept(pub Arc<dyn LintStorePort>);

/// A store that keeps what a run gives it in memory, for as long as it lives.
pub(crate) fn in_memory() -> Kept {
    Kept(Arc::new(ryo_core::outbound::lint_store::factory::memory()))
}

/// This engine's store over ruff beside main's, on the rules main's engine ran: main's figure, recorded once, and this one's.
pub(crate) struct AgainstMain {
    /// The bytes main's engine kept, as `fixtures/measured.toml` records them.
    pub main: u64,
    /// The bytes this engine keeps.
    pub here: u64,
    /// What a gate says when this one is the bigger: both figures, main's commit and date, and the rules left out.
    pub said: String,
}

/// The store this engine keeps over ruff with the rules main's engine ran (`fixtures/measured.toml`: its commit, the command and
/// the date it was measured), Rust and TOML linked as main linked them, beside main's figure. The rules main could not run are
/// left out of both sides, and named.
///
/// # Panics
///
/// When the measurement cannot be read, a rule it names is no recording here, or the rules do not run.
pub(crate) fn against_main() -> AgainstMain {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/measured.toml");
    let measured: toml::Table = std::fs::read_to_string(&path)
        .expect("the recorded measurement")
        .parse()
        .expect("the recorded measurement is TOML");
    let main = measured
        .get("main")
        .and_then(toml::Value::as_table)
        .expect("the measurement of main");
    let text = |key: &str| {
        main.get(key)
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("the measurement names its `{key}`"))
            .to_owned()
    };
    let (commit, date) = (text("commit"), text("measured"));
    let bytes = main
        .get("bytes")
        .and_then(toml::Value::as_integer)
        .and_then(|bytes| u64::try_from(bytes).ok())
        .expect("the measured bytes");
    let names: Vec<String> = main
        .get("rules")
        .and_then(toml::Value::as_array)
        .expect("the rules measured")
        .iter()
        .filter_map(|name| name.as_str().map(str::to_owned))
        .collect();
    let dropped: Vec<String> = main
        .get("dropped")
        .and_then(toml::Value::as_array)
        .expect("the rules left out")
        .iter()
        .filter_map(|each| {
            each.get("rule")
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    let rules: Vec<ryo_core::inbound::use_cases::lint::types::Rule> = super::base::recordings()
        .iter()
        .filter(|each| names.contains(&each.name))
        .map(super::base::rule_of)
        .collect();
    assert_eq!(
        rules.len(),
        names.len(),
        "every rule main ran is a recording here"
    );
    let copy = super::ruff::ruff_copy();
    let _cold = super::base::lint_report_with_languages(
        copy.path(),
        rules,
        Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
        super::base::store_at(copy.path()),
        Vec::new(),
        false,
        vec![ryo_core::outbound::language::factory::LanguageConnection::from(ryo_toml::linked())],
    )
    .expect("the rules run");
    let size = std::fs::metadata(copy.path().join(".ryo_cache/lint.store"))
        .expect("the store")
        .len();
    eprintln!("STORE {size} bytes, main's {bytes}");
    AgainstMain {
        main: bytes,
        here: size,
        said: format!(
            "the store over ruff with the {} rules main ran is {size} bytes, more than main's {bytes} (main {commit}, measured {date}); left out of both sides: {dropped:?}",
            names.len()
        ),
    }
}

/// The counting observer a run that reports its work is handed: the rows each step is fed and what each file hands the steps after
/// the files.
pub(crate) fn counter()
-> Arc<dyn ryo_execution_runtime::outbound::observer::interface::ObserverPort> {
    Arc::new(ryo_execution_runtime::outbound::observer::factory::counting())
}
