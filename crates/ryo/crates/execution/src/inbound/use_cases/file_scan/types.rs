//! The types of the `file_scan` use case: what the edge composed for the new engine, what a load is asked, the rule sets a load
//! made, a worker's state, the file a worker scans and what its scan answered.

use ryo_execution_runtime::application::file_scan::types::{EmitColumns, PlaceColumns, ScanWorker};
use ryo_execution_runtime::application::pack_load::types::HarnessLoad;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_execution_runtime::outbound::engine::interface::EnginePort;
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::types::WalkEngine;
use ryo_operators::spike::EngineMode;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::descriptions::types::{KindVocabularies, LanguageName};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::scan_rows::types::ScanFault;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::StoredPack;
use std::sync::Arc;

/// What the edge composed for the new engine: the engine, when it composed one, the digest its packs are keyed with, the walk and
/// the kinds of every linked language, in the order they were linked, which of them the request's rules are lowered against, and
/// which engines run the files.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::{FileScanSetting, ScanEngine, ScanLanguage};
/// use ryo_operators::spike::EngineMode;
/// use ryo_values::domain::descriptions::types::KindVocabularies;
/// use std::sync::Arc;
///
/// let setting = FileScanSetting {
///     engine: ScanEngine::Composed(Arc::new(ryo_execution_runtime::outbound::engine::factory::use_case())),
///     digest: Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
///     walks: Vec::new(),
///     vocabularies: KindVocabularies(Vec::new()),
///     language: ScanLanguage::EveryLinked,
///     mode: EngineMode::Old,
/// };
/// assert_eq!(setting.language, ScanLanguage::EveryLinked);
/// ```
pub struct FileScanSetting {
    /// The new engine, when the edge composed one.
    pub engine: ScanEngine,
    /// What a rule set's pack key is digested with.
    pub digest: Arc<dyn DigestPort>,
    /// The walk of every linked language, in the order they were linked. A language linked without its walk has a walk with no
    /// kind.
    pub walks: Vec<Arc<dyn WalkEngine>>,
    /// The kinds of every linked language, in the order they were linked.
    pub vocabularies: KindVocabularies,
    /// Which of them the request's rules are lowered against.
    pub language: ScanLanguage,
    /// Which engines run the files of a request.
    pub mode: EngineMode,
}

impl std::fmt::Debug for FileScanSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The new engine an edge composed for its lint, or that it composed none.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::ScanEngine;
///
/// assert!(matches!(ScanEngine::NotComposed, ScanEngine::NotComposed));
/// ```
#[derive(Clone)]
pub enum ScanEngine {
    /// The edge composed no engine: only the old engine can run its files.
    NotComposed,
    /// The engine the edge composed.
    Composed(Arc<dyn EnginePort>),
}

impl std::fmt::Debug for ScanEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotComposed => stringify!(NotComposed),
            Self::Composed(_) => stringify!(Composed),
        })
    }
}

/// Which linked languages a request's rules are lowered against, one rule set each.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::ScanLanguage;
/// use ryo_values::domain::descriptions::types::LanguageName;
///
/// assert_ne!(ScanLanguage::EveryLinked, ScanLanguage::Named(LanguageName::default()));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanLanguage {
    /// Every language linked.
    EveryLinked,
    /// The language the request says reads every file.
    Named(LanguageName),
}

/// What a load is asked before the files of one request: the selected rules, compiled, with what each binds its parameters to,
/// what the run was given as one digest, and the packs the store kept.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::ScanLoadRequest;
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let request = ScanLoadRequest { plans: &[], bindings: &[], given: Digest([0; 16]), stored: &[] };
/// assert!(request.stored.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ScanLoadRequest<'a> {
    /// The plan of each selected rule, in rule order.
    pub plans: &'a [&'a Plan],
    /// What each selected rule binds its plan's parameters to, in rule order: what the compilation bound.
    pub bindings: &'a [Vec<ParameterBinding>],
    /// What the run was given, as one digest: its rules as they were written, their fragments, what the host declared and which
    /// were selected. Made before any compile; a rule set's pack is kept under it.
    pub given: Digest,
    /// The packs the store kept.
    pub stored: &'a [StoredPack],
}

/// What the store is asked before anything is compiled: what the run was given as one digest, how many rules it selects, and
/// the packs the store kept.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::ScanStoredRequest;
/// use ryo_relations::domain::relations::types::Count;
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let request = ScanStoredRequest { given: Digest([0; 16]), selected: Count(0), stored: &[] };
/// assert!(request.stored.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ScanStoredRequest<'a> {
    /// What the run was given, as one digest.
    pub given: Digest,
    /// How many rules the request selects.
    pub selected: ryo_relations::domain::relations::types::Count,
    /// The packs the store kept.
    pub stored: &'a [StoredPack],
}

/// One rule set of a request: what the new engine's load made of the selected rules for one linked language (the rules it
/// accepted, their programs, and the pack the store keeps), that language's walk, and the columns its files' rows are read and
/// placed by.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::ScanSession;
///
/// assert!(ScanSession::default().rule_sets.is_empty());
/// ```
pub struct RuleSet {
    /// What the load made of the selected rules for the language.
    pub load: HarnessLoad,
    /// The language's walk.
    pub walk: Arc<dyn WalkEngine>,
    /// The columns of each selected rule's rows, by rule ordinal.
    pub columns: EmitColumns,
    /// The columns that place each selected rule's rows, by rule ordinal: read from the rule set's pack, as what a row of each
    /// rule emits is.
    pub places: PlaceColumns,
}

impl std::fmt::Debug for RuleSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The rule sets of a request, loaded once before its files: one for each linked language its rules were lowered against, in the
/// order the languages were linked. None under the old engine alone.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::ScanSession;
///
/// assert!(ScanSession::default().rule_sets.is_empty());
/// ```
#[derive(Debug, Default)]
pub struct ScanSession {
    /// The rule sets.
    pub rule_sets: Vec<RuleSet>,
}

/// A rule set of a session, by its place among the session's.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::RuleSetNumber;
///
/// assert!(RuleSetNumber(0) < RuleSetNumber(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuleSetNumber(pub usize);

/// One worker's state for a session: a file-scan worker for each of its rule sets, each cleared per file and never freed.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::FileScanWorker;
///
/// assert!(FileScanWorker::default().workers.is_empty());
/// ```
#[derive(Debug, Default)]
pub struct FileScanWorker {
    /// The worker of each rule set, in the session's order.
    pub workers: Vec<ScanWorker>,
}

/// A file a worker scans: its path, its number among the run's files, and its bytes.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::FileToScan;
/// use ryo_language::domain::files::types::SourcePath;
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_relations::domain::relations::types::FileId;
///
/// let file = FileToScan { path: SourcePath("a.txt"), file: FileId(0), source: ByteSlice(b"") };
/// assert_eq!(file.file, FileId(0));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FileToScan<'a> {
    /// The file's path, relative to the root.
    pub path: SourcePath<'a>,
    /// The file's number among the run's files.
    pub file: FileId,
    /// The file's bytes.
    pub source: ByteSlice<'a>,
}

/// What the scan of one file answered. The rows it found are in the buffer the worker was handed, whatever it answered.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::{RuleSetNumber, Scanned};
///
/// assert_ne!(Scanned::Whole(RuleSetNumber(0)), Scanned::Whole(RuleSetNumber(1)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scanned {
    /// The language of this rule set walked the file, and every rule's rows of it are complete.
    Whole(RuleSetNumber),
    /// The language of this rule set walked the file or failed on it, and the file's rows are not complete: why.
    Faulted(RuleSetNumber, ScanFault),
    /// No linked language of the session walks files like it: why.
    NotWalked(ScanFault),
}
