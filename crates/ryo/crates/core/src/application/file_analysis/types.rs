//! State owned by a bounded file analysis invocation.
use super::errors::FileAnalysisError;
use crate::domain::workers::types::Tables;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_execution::inbound::use_cases::execution::interface::RequestExecution;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
/// Whether the rule results of one file generation are all there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileCompleteness {
    /// Every rule result is whole.
    Complete,
    /// A rule result is missing, so no warm or edit record may be made of the file.
    Incomplete,
}

/// Whether a rule runs on one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleGate {
    /// The rule runs.
    Open,
    /// The rule does not run: its applicability is not met or not known.
    Blocked,
}

/// What one worker owns while the jobs run: its tables, its handle of the fact store and the first thing that went wrong in it.
pub(crate) struct Worker {
    /// What the worker owns besides its handle.
    pub tables: Tables,
    /// Where the rows of the file being read and the words kept of it are.
    pub facts: Box<dyn FactsWorker>,
    /// One bounded persisted-frontier reader for this worker's increasing jobs.
    pub old_rows: Option<Box<dyn crate::outbound::lint_store::interface::OldRowBatch>>,
    /// The failure that stopped the worker, when one did.
    pub failure: Option<FileAnalysisError>,
}

/// What a visit of a file reads that is the same for every file, and the ports a visit reaches besides.
pub(crate) struct FileVisit<'a> {
    /// One run's snapshot owns temporary normalized spans beyond any pre-read worker lifetime.
    pub store: Option<&'a dyn crate::outbound::lint_store::interface::ReadStore>,
    /// What the domain reads.
    pub setting: RequestVisitSetting<'a>,
}

use crate::domain::visit::types::{Noting, RoleRead};
use crate::outbound::language::types::{NoteWriting, RoleWriting};
impl From<Noting> for NoteWriting {
    fn from(noting: Noting) -> Self {
        match noting {
            Noting::Wanted => Self::Wanted,
            Noting::Unwanted => Self::Unwanted,
        }
    }
}
impl From<RoleRead> for RoleWriting {
    fn from(read: RoleRead) -> Self {
        match read {
            RoleRead::Reads => Self::Wanted,
            RoleRead::Skips => Self::Unwanted,
        }
    }
}

/// What a visit reads that the domain reads, with the request's provider, program and execution capabilities.
#[derive(Clone)]
pub(crate) struct RequestVisitSetting<'a> {
    /// Explicit request provider; none preserves ordinary path recognition.
    pub source_language: Option<&'a ryo_values::domain::descriptions::types::LanguageName>,
    /// One prepared executable program shared by all scopes in this request.
    pub program: Arc<PreparedExecutionProgram>,
    /// Required application capabilities; no domain code owns or constructs them.
    pub services: &'a RequestExecution,
    /// What the domain reads of every file.
    pub domain: crate::domain::workers::types::VisitSetting<'a>,
}

impl<'a> Deref for RequestVisitSetting<'a> {
    type Target = crate::domain::workers::types::VisitSetting<'a>;
    fn deref(&self) -> &Self::Target {
        &self.domain
    }
}

impl DerefMut for RequestVisitSetting<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.domain
    }
}

/// What the visits of a run read that is the same for every file: the files, how the rows are answered, the files whose findings
/// are written and what each rule is.
pub(crate) struct StreamRun<'a> {
    /// The files, in the order of their paths.
    pub files: &'a [ryo_values::domain::files::types::CorpusInput],
    /// How the rows the rules emit are answered.
    pub answering: &'a crate::domain::render::types::Answering,
    /// Only findings in these files are written; none means every file.
    pub only: &'a [ryo_language::domain::files::types::FilePath],
}

/// The findings waiting for their turn to leave, and the workers waiting for it.
pub(crate) struct Streaming {
    /// What waits, and how much is being read.
    pub ordered: std::sync::Mutex<crate::domain::ordering::types::Ordered>,
    /// Signalled when a file's findings left or its bytes were released.
    pub turn: std::sync::Condvar,
}

/// What the workers read together while the files are visited: the files, what every visit reads, where the findings wait for their
/// turn and how many were written.
pub(crate) struct Course<'a> {
    /// The files, in the order of their paths.
    pub files: &'a [ryo_values::domain::files::types::CorpusInput],
    /// What every visit reads.
    pub visit: &'a FileVisit<'a>,
    /// Where the findings wait for their turn.
    pub streaming: &'a Streaming,
    /// Only findings in these files are written; none means every file.
    pub only: &'a [ryo_language::domain::files::types::FilePath],
    /// How many findings the workers wrote.
    pub counted: &'a std::sync::atomic::AtomicU64,
    /// The new engine's part of the run: what each file's rows are compared with, when the run compares the two engines.
    pub scanning: &'a Scanning<'a>,
}

/// What the project is made from: the files listed, the session, the manifests, the relations the rules read, and what the rules
/// need of the project's partly read relations.
pub(crate) struct ProjectMaking<'a> {
    /// The files, in the order of their paths.
    pub files: &'a [ryo_values::domain::files::types::CorpusInput],
    /// Where the run reads the repository.
    pub session: &'a ryo_values::domain::runs::types::Session,
    /// The manifests the project reads.
    pub manifests: &'a [ryo_language::inbound::use_cases::project::types::ManifestFile],
    /// The relations the rules read.
    pub wanted: &'a [ryo_relations::domain::relations::types::Relation],
    /// What each rule needs of the files.
    pub needs: &'a [ryo_execution::inbound::use_cases::metadata::interface::Needs],
    /// The rules' ids.
    pub ids: &'a [ryo_values::domain::runs::types::RuleId],
}

/// What core read of a file before its rules run: the language port that walks it, its path, what core said of its own rows, how
/// much of it is whole and which relations are admitted.
#[derive(Clone, Copy)]
pub(crate) struct FileRead<'a> {
    /// The port the language that read the file is reached through.
    pub language: &'a dyn crate::outbound::language::interface::LanguagePort,
    /// The file's path, relative to the root.
    pub path: &'a ryo_language::domain::files::types::FilePath,
    /// What core said is read in part of a failure.
    pub said: &'a [ryo_values::domain::stored_facts::types::ReadInPart],
    /// Whether what was read is the whole file or a recovery.
    pub recovery: ryo_execution::inbound::use_cases::metadata::interface::Recovery,
    /// The relations admitted, which count towards the file's partiality.
    pub active: &'a [ryo_relations::domain::relations::types::Relation],
    /// The engine's own rows the file's rules may read.
    pub engine: EngineFile<'a>,
}

/// The engine's own rows of a file, before they are read: which the file's rules ask for, its executable bit as the input edge
/// gave it, and the detail of a disk-read failure, when that is why the file has no source.
#[derive(Clone, Copy)]
pub(crate) struct EngineFile<'a> {
    /// Which engine rows the file's rules ask for.
    pub asked: crate::domain::reading::types::EngineAsked,
    /// The file's executable bit, as the input edge gave it.
    pub executable: ryo_values::domain::files::types::ExecutablePermission,
    /// What the disk said when it could not be read.
    pub failure: Option<crate::domain::reading::types::FailureDetail<'a>>,
}

/// The engine's own rows of a file, as the source of them: it writes those the run demands of what the file's rules ask for.
pub(crate) struct EngineSource<'a> {
    /// The relations the engine makes: the catalog's last.
    pub serves: &'a [ryo_relations::domain::relations::types::Relation],
    /// Which of them the file's rules ask for.
    pub asked: crate::domain::reading::types::EngineAsked,
    /// The file.
    pub file: ryo_relations::domain::relations::types::FileId,
    /// Its bytes, whose lines are counted.
    pub bytes: &'a ryo_values::domain::files::types::FileBytes,
    /// Its executable bit, as the input edge gave it.
    pub executable: ryo_values::domain::files::types::ExecutablePermission,
    /// What the disk said when it could not be read.
    pub failure: Option<crate::domain::reading::types::FailureDetail<'a>>,
}

/// The language that read a file, as the source of its rows and its tree: it walks the file for what the run asks, admits what it
/// wrote, and keeps the refusal it met for core to raise unchanged.
pub(crate) struct LanguageSource<'a> {
    /// The port the language is reached through.
    pub language: &'a dyn crate::outbound::language::interface::LanguagePort,
    /// Which linked language read the file.
    pub number: ryo_values::domain::stored_facts::types::LanguageNumber,
    /// The file as the language read it.
    pub file: crate::outbound::language::types::FileToRead<'a>,
    /// The tree the language read the file into, the worker's own.
    pub tree: &'a mut ryo_language::domain::elements::types::Elements,
    /// The relations a language's walk makes: every relation but the engine's.
    pub serves: &'a [ryo_relations::domain::relations::types::Relation],
    /// The refusal of the language's output, kept with its reason.
    pub refused: &'a mut Option<FileAnalysisError>,
}

/// The new engine's part of a run: the use case that scans the files, the rule sets it loaded for the request, and what a
/// scanned file's rows are rendered and compared with. Under the old engine alone it holds no rule set, and nothing reads it.
pub(crate) struct Scanning<'a> {
    /// Scans a request's files with the new engine.
    pub scan: Box<dyn ryo_execution::inbound::use_cases::file_scan::interface::FileScanUseCase>,
    /// The rule sets loaded for the request.
    pub session: ryo_execution::inbound::use_cases::file_scan::types::ScanSession,
    /// What a row of each rule emits, in rule order: its subject, its key and its headings.
    pub emits: Vec<ryo_values::domain::query_plans::types::Emit>,
    /// The rules' ids.
    pub ids: &'a [ryo_values::domain::runs::types::RuleId],
    /// Which rule governs which file.
    pub placed: &'a ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes,
    /// The path of each file, by file number.
    pub paths: Vec<ryo_language::domain::files::types::FilePath>,
    /// How the rows the rules emit are answered.
    pub answering: &'a crate::domain::render::types::Answering,
    /// The files the run lists, in the order of their paths.
    pub files: &'a [ryo_values::domain::files::types::CorpusInput],
    /// The store the run read beside its files, when it read one: where the packs of its rule sets are kept.
    pub read: Option<&'a Arc<dyn crate::outbound::lint_store::interface::ReadStore>>,
    /// What the reasons of the languages linked are called.
    pub unknown: Vec<ryo_relations::domain::terms::types::UnknownName>,
}

/// What one worker owns while the new engine scans the files it takes: the use case's state for the worker, the file's bytes,
/// the rows the scan handed, their rendering, which rules govern the file, and what the worker could not read or decide.
#[derive(Default)]
pub(crate) struct ScanState {
    /// The use case's state for the worker.
    pub worker: ryo_execution::inbound::use_cases::file_scan::types::FileScanWorker,
    /// The bytes of the file being scanned.
    pub bytes: ryo_values::domain::files::types::FileBytes,
    /// The rows the scan of the file handed.
    pub rows: Vec<ryo_values::domain::scan_rows::types::ScanRow>,
    /// The rule set whose language walked the file, when one did.
    pub by: Option<ryo_execution::inbound::use_cases::file_scan::types::RuleSetNumber>,
    /// Where the lines of the file start, as far as its last row.
    pub starts: crate::domain::finish::types::LineStarts,
    /// The file's rows, rendered.
    pub rendered: crate::domain::render::types::Rendered,
    /// The rules that govern the file.
    pub mask: ryo_execution::inbound::use_cases::metadata::interface::Mask,
    /// The globs that matched the file.
    pub hits: ryo_values::domain::files::types::Hits,
    /// The names of a file the new engine scanned: none, a kept row holds its text.
    pub symbols: ryo_language::domain::symbols::types::Symbols,
    /// The tree of a file the new engine scanned, as the old engine holds one: none.
    pub elements: ryo_language::domain::elements::types::Elements,
    /// The files no language walked, with why.
    pub unread: Vec<ryo_values::domain::records::types::Unread>,
    /// What the rules could not decide.
    pub unknowns: Vec<ryo_execution::inbound::use_cases::metadata::interface::Unknown>,
    /// The failure that stopped the worker, when one did.
    pub failure: Option<FileAnalysisError>,
}

/// What a run the new engine scanned alone answers once its rows are written: the files no language walked, what the rules could
/// not decide, and how many rows the answer has and shows.
pub(crate) struct ScannedRun {
    /// The files no language walked, in the order of their paths.
    pub unread: Vec<ryo_values::domain::records::types::Unread>,
    /// What the rules could not decide.
    pub unknowns: Vec<ryo_execution::inbound::use_cases::metadata::interface::Unknown>,
    /// How many rows the answer has and how many were written.
    pub counts: crate::outbound::row_sink::types::AnswerCounts,
}

/// What the new engine's part of a run is made of, before its rule sets are loaded.
pub(crate) struct ScanRequest<'a, 'l> {
    /// The language the request says reads every file, when it names one.
    pub named: Option<&'l ryo_values::domain::descriptions::types::LanguageName>,
    /// The plan of each rule, in rule order: read while the rule sets are loaded.
    pub plans: &'l [&'l ryo_values::domain::query_plans::types::Plan],
    /// The digest of each rule, in rule order: read while the rule sets are loaded.
    pub digests: &'l [ryo_values::domain::stored_facts::types::Digest],
    /// The rules' ids.
    pub ids: &'a [ryo_values::domain::runs::types::RuleId],
    /// Which rule governs which file.
    pub placed: &'a ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes,
    /// How the rows the rules emit are answered.
    pub answering: &'a crate::domain::render::types::Answering,
    /// The files the run lists, in the order of their paths.
    pub files: &'a [ryo_values::domain::files::types::CorpusInput],
    /// The store the run read beside its files, when it read one.
    pub read: Option<&'a Arc<dyn crate::outbound::lint_store::interface::ReadStore>>,
}

/// Where the two engines' rows differ, as the harness says it: the rule, the file, and the first row each engine has that the
/// other has not there; none for the engine that has no more rows.
#[derive(Debug)]
pub(crate) struct RowsDiffer<'a> {
    /// The rule's id.
    #[expect(
        dead_code,
        reason = "read by the harness's sentence, which is this value's `Debug`"
    )]
    pub rule: Option<&'a ryo_values::domain::runs::types::RuleId>,
    /// The file's path.
    #[expect(
        dead_code,
        reason = "read by the harness's sentence, which is this value's `Debug`"
    )]
    pub file: Option<&'a ryo_language::domain::files::types::FilePath>,
    /// The old engine's row.
    #[expect(
        dead_code,
        reason = "read by the harness's sentence, which is this value's `Debug`"
    )]
    pub old: Option<crate::domain::found_rows::types::ComparedRow>,
    /// The new engine's row.
    #[expect(
        dead_code,
        reason = "read by the harness's sentence, which is this value's `Debug`"
    )]
    pub new: Option<crate::domain::found_rows::types::ComparedRow>,
}
