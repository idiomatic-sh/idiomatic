//! The types of the `file_scan` operation: a worker's state, the columns a found row is read and placed by, and the run's report
//! of the rows kept as owned text and numbers.

use super::errors::FileScanError;
use ryo_engine::domain::findings::types::{RuleColumns, ScanCounters, WorkNotice};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::scratch::types::ScanScratch;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::inbound::use_cases::walk::types::WalkEngine;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::scan_rows::types::{FrontierRow, ScanRow};
use std::sync::Arc;

/// The columns of every selected rule, by rule ordinal: read from the engine's rule set, once for the run.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::EmitColumns;
///
/// assert!(EmitColumns::default().0.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EmitColumns(pub Vec<RuleColumns>);

/// The columns that say where a rule's row is, for every selected rule, by rule ordinal: its key columns, then its subject.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::PlaceColumns;
///
/// assert!(PlaceColumns::default().0.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaceColumns(pub Vec<Vec<ColumnIndex>>);

/// A file whose scan faulted, and why.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::errors::FileScanError;
/// use ryo_execution_runtime::application::file_scan::types::FaultedFile;
/// use ryo_relations::domain::relations::types::FileId;
///
/// let faulted = FaultedFile { file: FileId(2), fault: FileScanError::NotLent };
/// assert_eq!(faulted.file, FileId(2));
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct FaultedFile {
    /// The file.
    pub file: FileId,
    /// Why its scan faulted.
    pub fault: FileScanError,
}

/// What the engine said of a rule's work in a scanned file: the file, and the engine's notice. The rule's rows of the file
/// are complete all the same.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{HeaviestWork, InstructionCount, WorkNotice, WorkVerdict};
/// use ryo_execution_runtime::application::file_scan::types::FileNotice;
/// use ryo_relations::domain::relations::types::FileId;
/// use ryo_values::domain::plan_parts::types::StepId;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let notice = WorkNotice {
///     rule: SelectedRuleOrdinal(0),
///     verdict: WorkVerdict::Over,
///     work: InstructionCount(9),
///     allowance: InstructionCount(1),
///     step: StepId(0),
///     heaviest: HeaviestWork::Sorted { rows: InstructionCount(9) },
/// };
/// assert_eq!(FileNotice { file: FileId(2), notice }.file, FileId(2));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileNotice {
    /// The file.
    pub file: FileId,
    /// What the engine said of a rule's work in it.
    pub notice: WorkNotice,
}

/// What a worker, or the run, found: the findings, the frontier rows in the order the files handed them, the faulted files,
/// what the engine noticed of the rules' work, and what the scans counted together.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::ScanReport;
///
/// assert!(ScanReport::default().found.is_empty());
/// assert!(ScanReport::default().notices.is_empty());
/// ```
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScanReport {
    /// The findings.
    pub found: Vec<ScanRow>,
    /// The frontier rows.
    pub frontier: Vec<FrontierRow>,
    /// The files whose scan faulted.
    pub faults: Vec<FaultedFile>,
    /// What the engine noticed of a rule's work in a file, in the order it said it.
    pub notices: Vec<FileNotice>,
    /// What the scans that answered counted, together.
    pub counted: ScanCounters,
}

/// Whether a worker's state counts what its scans do: a measuring run's does, for the work ledger; every other run's is the
/// plain state, which counts nothing beyond the scan's own counters.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::ScanMeasuring;
///
/// assert_ne!(ScanMeasuring::Plain, ScanMeasuring::Measuring);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanMeasuring {
    /// The plain state.
    Plain,
    /// The state that counts.
    Measuring,
}

/// One worker's state: the language, the node table, the language's own buffers and the engine's per-file state, each
/// cleared per file and never freed, and what the worker found. Made by `service::worker`.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::service::worker;
/// use ryo_execution_runtime::application::file_scan::types::ScanMeasuring;
/// use ryo_language::inbound::use_cases::walk::types::WalkEngine;
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
/// use std::sync::Arc;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let language: Arc<dyn WalkEngine> = Arc::new(WalkUseCaseImpl::new());
/// assert!(worker(&language, &engine, &Programs::default(), ScanMeasuring::Plain).report.found.is_empty());
/// ```
pub struct ScanWorker {
    /// The language whose files the worker scans.
    pub language: Arc<dyn WalkEngine>,
    /// The table of the file being scanned.
    pub table: NodeTable,
    /// The language's own buffers for this worker.
    pub buffers: Box<dyn std::any::Any + Send>,
    /// The engine's per-file state.
    pub scan: ScanScratch,
    /// What the worker found in the files it scanned.
    pub report: ScanReport,
    /// SPIKE (ryo-fmx3.1, E): what the host knows of the file in hand that no language reads: whether it may be executed.
    /// Set by whoever hands the worker a file, before the scan; unknown until then.
    pub executable: ryo_values::domain::files::types::ExecutablePermission,
    /// SPIKE (ryo-fmx3.1): the project's rows the host holds, answered to each file's scan around the language's view; a
    /// file no language walks is scanned with no node table when they hold a row about it. None until the host hands them.
    pub hosted: Arc<super::hosted::types::HostedRows>,
    /// SPIKE (ryo-fmx3.1): what the host read of the file in hand, set with `executable`.
    pub read: HostRead,
}

/// SPIKE (ryo-fmx3.1): what the host read of a file it hands the scan, as today's engine is told it: its bytes, a manifest no
/// language walks (whose rows are the host's), or nothing, with the reason; a file that could not be read is still scanned,
/// with no node table, so that the rules that ask for it run.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::HostRead;
///
/// assert_eq!(HostRead::default(), HostRead::Source);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HostRead {
    /// The file's bytes were read.
    #[default]
    Source,
    /// The bytes of a manifest of a linked language were read; no language walks it.
    Manifest,
    /// The file could not be read.
    Failed {
        /// Why, as the host says it.
        detail: Box<str>,
        /// Whether the host saw the file at all: one the request names and the disk lacks was not.
        observed: ObservedPath,
    },
}

/// SPIKE (ryo-fmx3.1): whether the host saw a file it could not read.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::ObservedPath;
///
/// assert_ne!(ObservedPath::Observed, ObservedPath::Unobserved);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedPath {
    /// The host saw the file.
    Observed,
    /// The file the request names is not there.
    Unobserved,
}

impl std::fmt::Debug for ScanWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// What every file of a run is scanned with: the rule set, the columns of its rules' rows as the engine says them
/// (`service::emit_columns`), and the columns that place a row, as the rule set's pack says them (`service::place_columns`).
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::types::{EmitColumns, PlaceColumns, ScanRun};
///
/// let (programs, columns, places) = (Programs::default(), EmitColumns::default(), PlaceColumns::default());
/// let run = ScanRun { programs: &programs, columns: &columns, places: &places };
/// assert!(run.places.0.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ScanRun<'a> {
    /// The rule set.
    pub programs: &'a Programs,
    /// The columns of each selected rule's rows, by rule ordinal.
    pub columns: &'a EmitColumns,
    /// The columns that place each selected rule's rows, by rule ordinal.
    pub places: &'a PlaceColumns,
}

/// One line of a measuring run's work ledger, as the harness prints it: a unit, its work class, its counts and the rules
/// that reach it.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::LedgerLine;
///
/// assert!(LedgerLine("unit=Rule:0 class=Navigation rows=2 steps=8 nanos=0 idle_nanos=0 owners=0".into()).0.starts_with("unit="));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerLine(pub Box<str>);

/// One value of a ledger line, as it is written after its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LedgerValue(pub(crate) String);
