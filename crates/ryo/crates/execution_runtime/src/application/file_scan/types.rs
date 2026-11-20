//! The types of the `file_scan` operation: a worker's state, the columns a found row is read and placed by, and the run's report
//! of the rows kept as owned text and numbers.

use super::errors::FileScanError;
use ryo_engine::domain::findings::types::RuleColumns;
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

/// What a worker, or the run, found: the findings, the frontier rows in the order the files handed them, and the faulted files.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::ScanReport;
///
/// assert!(ScanReport::default().found.is_empty());
/// ```
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScanReport {
    /// The findings.
    pub found: Vec<ScanRow>,
    /// The frontier rows.
    pub frontier: Vec<FrontierRow>,
    /// The files whose scan faulted.
    pub faults: Vec<FaultedFile>,
}

/// One worker's state: the language, the node table, the language's own buffers and the engine's per-file state, each
/// cleared per file and never freed, and what the worker found. Made by `service::worker`.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::service::worker;
/// use ryo_language::inbound::use_cases::walk::types::WalkEngine;
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
/// use std::sync::Arc;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let language: Arc<dyn WalkEngine> = Arc::new(WalkUseCaseImpl::new());
/// assert!(worker(&language, &engine, &Programs::default()).report.found.is_empty());
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
