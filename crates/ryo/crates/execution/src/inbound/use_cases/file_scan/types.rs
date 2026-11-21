//! The types of the `file_scan` use case: what the edge composed for the new engine, what a load is asked, the rule sets a load
//! made, a worker's state, the file a worker scans, what its scan answered and noticed, and what a run's workers counted.

use ryo_execution_runtime::application::file_scan::types::{
    EmitColumns, FileNotice, LedgerLine, PlaceColumns, ScanWorker,
};
use ryo_execution_runtime::application::pack_load::types::HarnessLoad;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_execution_runtime::outbound::engine::interface::EnginePort;
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::types::WalkEngine;
use ryo_operators::spike::EngineMode;
use ryo_relations::domain::relations::types::{Count, FileId};
use ryo_values::domain::descriptions::types::{KindVocabularies, LanguageName};
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::scan_rows::types::ScanFault;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::StoredPack;
use std::sync::Arc;

/// What the edge composed for the new engine: the engine, when it composed one, the digest its packs are keyed with, the walk and
/// the kinds of every linked language, in the order they were linked, which of them the request's rules are lowered against,
/// which engines run the files, what a scan does when a rule passes its work allowance, and whether the run measures.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::{
///     FileScanSetting, Overruns, ScanEngine, ScanLanguage, ScanMeasure,
/// };
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
///     overruns: Overruns::Report,
///     measure: ScanMeasure::Off,
/// };
/// assert_eq!(setting.overruns, Overruns::Report);
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
    /// What a file's scan does when a rule passes its work allowance in the file.
    pub overruns: Overruns,
    /// Whether the workers count what their scans do, for the work ledger.
    pub measure: ScanMeasure,
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

/// What a file's scan does when a rule passes its work allowance in the file. Neither stops the rule: its rows of the file
/// are complete, and what the engine noticed is handed over either way.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::Overruns;
///
/// assert_ne!(Overruns::Report, Overruns::Fail);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overruns {
    /// A user's run: the scan answers as it would without the overrun.
    Report,
    /// One of our own gates: the scan answers that the file overran, and the run fails on it.
    Fail,
}

/// Whether a run measures: its workers then count what their scans do, and its work ledger is read after its files.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::ScanMeasure;
///
/// assert_ne!(ScanMeasure::Off, ScanMeasure::On);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanMeasure {
    /// The run does not measure.
    Off,
    /// The run measures.
    On,
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
/// the statistics they are lowered with, what the run was given as one digest, and the packs the store kept.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::ScanLoadRequest;
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let request = ScanLoadRequest { plans: &[], bindings: &[], statistics: ByteSlice(&[]), given: Digest([0; 16]), stored: &[], declared: Default::default() };
/// assert!(request.stored.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ScanLoadRequest<'a> {
    /// The plan of each selected rule, in rule order.
    pub plans: &'a [&'a Plan],
    /// What each selected rule binds its plan's parameters to, in rule order: what the compilation bound.
    pub bindings: &'a [Vec<ParameterBinding>],
    /// The statistics the rules are lowered with, as the bytes of the file that holds them: none, no statistics.
    pub statistics: ByteSlice<'a>,
    /// What the run was given, as one digest: its rules as they were written, their fragments, what the host declared, which
    /// were selected and the statistics they are lowered with. Made before any compile; a rule set's pack is kept under it.
    pub given: Digest,
    /// The packs the store kept.
    pub stored: &'a [StoredPack],
    /// What the host declares beside the plans: the rules as the run was given them and the relations it declares.
    pub declared: ryo_execution_runtime::application::pack_load::types::Declared<'a>,
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
    pub selected: Count,
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
/// use ryo_relations::domain::relations::types::{Count, FileId};
///
/// let file = FileToScan { path: SourcePath("a.txt"), file: FileId(0), source: ByteSlice(b""), executable: Default::default(), hosted: None };
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
    /// SPIKE (ryo-fmx3.1, E): whether the host saw the file as one that may be executed.
    pub executable: ryo_values::domain::files::types::ExecutablePermission,
    /// SPIKE (ryo-fmx3.1): the project's rows the host holds, the same for every file of the run; none when it holds none.
    pub hosted: Option<&'a std::sync::Arc<ryo_execution_runtime::application::file_scan::hosted::types::HostedRows>>,
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
    /// The language of this rule set walked the file and every rule's rows of it are complete; a rule passed its work
    /// allowance in it, and the scan is set to fail on that: what was noticed says which rule, and the run fails.
    Overran(RuleSetNumber),
}

/// What the engine noticed of a rule's work in a scanned file: the rule, the step of its plan that did the most work, and
/// the engine's notice as the file scan kept it, which `FileScanUseCase::worded` words.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::Noticed;
///
/// fn rule_of(noticed: &Noticed) -> u32 {
///     noticed.rule.0
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Noticed {
    /// The rule.
    pub rule: SelectedRuleOrdinal,
    /// The step of the rule's plan whose rows the unit that did the most work makes.
    pub step: StepId,
    /// The notice, with its file.
    pub kept: FileNotice,
}

/// What a session's workers counted over the files they scanned: the steps, the candidates, the rows of the tables made, the
/// rows the fills read and gave, and the rows the programs' iterations gave, in that order; and, for a run that measures,
/// its work ledger, one line a unit.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::types::ScanMeasured;
///
/// assert!(ScanMeasured::default().ledger.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanMeasured {
    /// The five counts, in the order said.
    pub counted: [Count; 5],
    /// The ledger's lines.
    pub ledger: Vec<LedgerLine>,
}
