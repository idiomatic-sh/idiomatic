//! The `file_scan` service. Each worker walks a file through its language and scans it through the engine; every found row is
//! kept as owned text and numbers, with where in the file it is, before the file's words and its table are gone; the run's report
//! is its workers' rows, sorted. Nothing here is done per rule for a file: a rule whose anchors are absent from a file costs the
//! host nothing, and a found row is looked up by its rule ordinal.

use super::errors::FileScanError;
use super::keeping::service::RowKeeper;
use super::types::{EmitColumns, FaultedFile, PlaceColumns, ScanReport, ScanRun, ScanWorker};
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::findings::types::{FoundTarget, InstructionCount, ScanCounters};
use ryo_engine::domain::programs::service::needs;
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::scratch::types::ScanScratch;
use ryo_engine::inbound::use_cases::scan::errors::ScanError;
use ryo_language::domain::facts::types::FileFacts;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::inbound::use_cases::walk::errors::WalkError;
use ryo_language::inbound::use_cases::walk::types::{FileVisit, WalkEngine, WalkRequest};
use ryo_values::domain::query_plans::types::Emit;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::scan_rows::types::{FaultOf, FaultSentence, ScanFault, ScanRow};
use std::sync::Arc;

/// The columns of each accepted rule's rows, asked of the engine once for the run and laid out by rule ordinal.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::service::emit_columns;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// assert!(emit_columns(&engine, &Programs::default()).0.is_empty());
/// ```
#[must_use]
pub fn emit_columns(engine: &dyn EnginePort, programs: &Programs) -> EmitColumns {
    let mut columns = EmitColumns::default();
    for (rule, held) in engine
        .accepted(programs)
        .into_iter()
        .zip(engine.columns(programs))
    {
        let place = usize::try_from(rule.0).unwrap_or(usize::MAX);
        if columns.0.len() <= place {
            columns
                .0
                .resize_with(place.saturating_add(1), Default::default);
        }
        if let Some(slot) = columns.0.get_mut(place) {
            *slot = held;
        }
    }
    columns
}

/// The columns that place a row of each rule a rule set runs, by rule ordinal: the key columns of what the rule emits, then its
/// subject. `rules` are the rules the rule set runs and `emits` what a row of each emits, in the same order, as the rule set's
/// pack says it: no plan is read.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::service::place_columns;
///
/// assert!(place_columns(&[], &[]).0.is_empty());
/// ```
#[must_use]
pub fn place_columns(rules: &[SelectedRuleOrdinal], emits: &[Emit]) -> PlaceColumns {
    let mut places = PlaceColumns::default();
    for (rule, emit) in rules.iter().zip(emits) {
        let place = usize::try_from(rule.0).unwrap_or(usize::MAX);
        if places.0.len() <= place {
            places.0.resize_with(place.saturating_add(1), Vec::new);
        }
        if let Some(slot) = places.0.get_mut(place) {
            *slot = emit
                .key
                .iter()
                .chain(std::iter::once(&emit.subject))
                .copied()
                .collect();
        }
    }
    places
}

/// A worker's state for a rule set, before its first file.
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
/// assert!(worker(&language, &engine, &Programs::default()).report.faults.is_empty());
/// ```
#[must_use]
pub fn worker(
    language: &Arc<dyn WalkEngine>,
    engine: &dyn EnginePort,
    programs: &Programs,
) -> ScanWorker {
    ScanWorker {
        language: Arc::clone(language),
        table: NodeTable::default(),
        buffers: language.buffers(),
        scan: engine.state_for(programs),
        report: ScanReport::default(),
    }
}

/// The visit that scans the file its language lends.
struct Visiting<'a> {
    /// The engine.
    engine: &'a dyn EnginePort,
    /// What every file of the run is scanned with.
    run: &'a ScanRun<'a>,
    /// The worker's per-file engine state.
    state: &'a mut ScanScratch,
    /// Where the worker keeps what it found.
    report: &'a mut ScanReport,
    /// The first target a row did not fit the columns of.
    misfit: Option<FoundTarget>,
    /// What the scan answered; nothing until the language lends the file.
    answer: Option<Result<ScanCounters, ScanError>>,
}

impl FileVisit for Visiting<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        let mut keeping = RowKeeper {
            columns: self.run.columns,
            places: self.run.places,
            file: file.file,
            table: file.table,
            report: &mut *self.report,
            misfit: &mut self.misfit,
        };
        self.answer = Some(
            self.engine
                .scan(self.run.programs, file, self.state, &mut keeping),
        );
    }
}

/// One file, in the worker that took it: walked by the worker's language into its table, lent to the engine's scan, its found
/// rows kept in the worker's report. Rows found before a fault stay kept: the fault says whose rows of the file are not complete.
fn scanned(
    engine: &dyn EnginePort,
    run: &ScanRun<'_>,
    worker: &mut ScanWorker,
    file: WalkRequest<'_>,
) -> Result<ScanCounters, FileScanError> {
    let mut visiting = Visiting {
        engine,
        run,
        state: &mut worker.scan,
        report: &mut worker.report,
        misfit: None,
        answer: None,
    };
    let (table, buffers) = (&mut worker.table, &mut *worker.buffers);
    let needs = needs(run.programs);
    let walked = worker
        .language
        .walk_file(file, needs, table, buffers, &mut visiting);
    match (walked, visiting.answer, visiting.misfit) {
        (Err(cause), _, _) => Err(FileScanError::Walk { cause }),
        (Ok(()), None, _) => Err(FileScanError::NotLent),
        (Ok(()), Some(Err(fault)), _) => Err(FileScanError::Rule { fault }),
        (Ok(()), Some(Ok(_)), Some(target)) => Err(FileScanError::Columns { target }),
        (Ok(()), Some(Ok(counted)), None) => Ok(counted),
    }
}

/// One file, in the worker that took it, its findings taken out of the worker: they are moved to `rows`, after what it holds,
/// sorted by rule, where they start and end, and cells. The frontier rows the file handed stay in the worker's report.
///
/// # Errors
///
/// The fault of the file's scan, as [`ran`] lists it; the rows found before it are in `rows` all the same.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::service::{taken, worker};
/// use ryo_execution_runtime::application::file_scan::types::{EmitColumns, PlaceColumns, ScanRun};
/// use ryo_language::domain::files::types::SourcePath;
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_language::inbound::use_cases::walk::types::{WalkEngine, WalkRequest};
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
/// use ryo_relations::domain::relations::types::FileId;
/// use std::sync::Arc;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let programs = Programs::default();
/// let language: Arc<dyn WalkEngine> = Arc::new(WalkUseCaseImpl::new());
/// let mut worker = worker(&language, &engine, &programs);
/// let file = WalkRequest { path: SourcePath("a.txt"), file: FileId(0), source: ByteSlice(b"") };
/// let (columns, places) = (EmitColumns::default(), PlaceColumns::default());
/// let run = ScanRun { programs: &programs, columns: &columns, places: &places };
/// let mut rows = Vec::new();
/// assert!(taken(&engine, &run, &mut worker, file, &mut rows).is_err());
/// ```
pub fn taken(
    engine: &dyn EnginePort,
    run: &ScanRun<'_>,
    worker: &mut ScanWorker,
    file: WalkRequest<'_>,
    rows: &mut Vec<ScanRow>,
) -> Result<ScanCounters, FileScanError> {
    let answered = scanned(engine, run, worker, file);
    let from = rows.len();
    rows.append(&mut worker.report.found);
    if let Some(found) = rows.get_mut(from..) {
        found.sort_unstable();
    }
    answered
}

/// A file's fault as it crosses to whoever answers the run: whose fault it is, and the sentence that says why.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::errors::FileScanError;
/// use ryo_execution_runtime::application::file_scan::service::crossed;
/// use ryo_values::domain::scan_rows::types::FaultOf;
///
/// assert_eq!(crossed(&FileScanError::NotLent).of, FaultOf::File);
/// ```
#[must_use]
pub fn crossed(fault: &FileScanError) -> ScanFault {
    let of = match fault {
        FileScanError::Walk {
            cause: WalkError::Unsupported,
        } => FaultOf::NoLanguage,
        FileScanError::Walk {
            cause: WalkError::Library,
        }
        | FileScanError::NotLent
        | FileScanError::Columns { .. } => FaultOf::File,
        FileScanError::Rule {
            fault: ScanError::Refused { rule, .. } | ScanError::Partial { rule, .. },
        } => FaultOf::Rule(*rule),
    };
    ScanFault {
        of,
        sentence: FaultSentence(fault.to_string().into()),
    }
}

/// The files one worker took, each scanned in order: a faulted file is listed in the worker's report with its fault (`Walk` when
/// the language does not read it or its parser fails, `NotLent`, `Rule` when a rule's program read a relation refused or filled
/// in part, `Columns` when a found row does not fit the columns the run holds) and the run goes on.
/// Answers what the scans counted together.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::service::{ran, worker};
/// use ryo_execution_runtime::application::file_scan::types::{EmitColumns, PlaceColumns, ScanRun};
/// use ryo_language::domain::files::types::SourcePath;
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_language::inbound::use_cases::walk::types::{WalkEngine, WalkRequest};
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
/// use ryo_relations::domain::relations::types::FileId;
/// use std::sync::Arc;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let programs = Programs::default();
/// let language: Arc<dyn WalkEngine> = Arc::new(WalkUseCaseImpl::new());
/// let mut worker = worker(&language, &engine, &programs);
/// let file = WalkRequest { path: SourcePath("a.txt"), file: FileId(0), source: ByteSlice(b"") };
/// let (columns, places) = (EmitColumns::default(), PlaceColumns::default());
/// let run = ScanRun { programs: &programs, columns: &columns, places: &places };
/// let _counted = ran(&engine, &run, &mut worker, &[file]);
/// assert_eq!(worker.report.faults.len(), 1);
/// ```
pub fn ran(
    engine: &dyn EnginePort,
    run: &ScanRun<'_>,
    worker: &mut ScanWorker,
    files: &[WalkRequest<'_>],
) -> ScanCounters {
    let mut total = ScanCounters::default();
    let sum = |one: InstructionCount, other: InstructionCount| {
        InstructionCount(one.0.saturating_add(other.0))
    };
    for file in files {
        match scanned(engine, run, worker, *file) {
            Ok(counted) => {
                total = ScanCounters {
                    steps: sum(total.steps, counted.steps),
                    candidates: sum(total.candidates, counted.candidates),
                    made_rows: sum(total.made_rows, counted.made_rows),
                };
            }
            Err(fault) => worker.report.faults.push(FaultedFile {
                file: file.file,
                fault,
            }),
        }
    }
    total
}

/// The run's report from its workers': the findings sorted by file, rule, where they start and end, and cells; the frontier rows by
/// file, rule and entry, in the order each file handed them; the faulted files by file.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::service::reported;
/// use ryo_execution_runtime::application::file_scan::types::ScanReport;
///
/// assert_eq!(reported([ScanReport::default(), ScanReport::default()]), ScanReport::default());
/// ```
#[must_use]
pub fn reported(workers: impl IntoIterator<Item = ScanReport>) -> ScanReport {
    let mut report = ScanReport::default();
    for each in workers {
        report.found.extend(each.found);
        report.frontier.extend(each.frontier);
        report.faults.extend(each.faults);
    }
    report.found.sort_unstable();
    report.frontier.sort_by(|one, other| {
        one.file
            .cmp(&other.file)
            .then(one.rule.cmp(&other.rule))
            .then(one.input.cmp(&other.input))
    });
    report.faults.sort_by_key(|faulted| faulted.file);
    report
}
