//! The `file_scan` service. Each worker walks a file through its language and scans it through the engine; every found row is
//! kept as owned text and numbers before the file's words are cleared; the run's report is its workers' rows, sorted. On the
//! build branch `both` compares that report's rows with the old engine's, per rule and file. Nothing here is done per rule for a
//! file: a rule whose anchors are absent from a file costs the host nothing.

use super::constants::CELL_SEPARATOR;
use super::errors::FileScanError;
use super::types::{
    EmitColumns, FaultedFile, FrontierRow, HarnessDifference, HarnessRows, KeptCell, KeptText,
    RowText, RuleFile, ScanReport, ScanRow, ScanRun, ScanWorker,
};
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::findings::types::{
    FoundCells, FoundRows, FoundTarget, InstructionCount, ScanCounters,
};
use ryo_engine::domain::programs::service::needs;
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::scratch::types::ScanScratch;
use ryo_engine::inbound::use_cases::scan::errors::ScanError;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::facts::service::text;
use ryo_language::domain::facts::types::{FileFacts, FileWords};
use ryo_language::domain::node_table::service::builder;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::inbound::use_cases::walk::interface::WalkUseCase;
use ryo_language::inbound::use_cases::walk::types::{FileVisit, WalkRequest};
use ryo_operators::spike::{engine_compared, engine_difference};
use ryo_relations::domain::relations::types::{CellKind, Count, FileId};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// A worker's state for a rule set, before its first file.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::service::worker;
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// assert!(worker::<WalkUseCaseImpl>(&engine, &Programs::default()).report.faults.is_empty());
/// ```
#[must_use]
pub fn worker<W: WalkUseCase>(engine: &dyn EnginePort, programs: &Programs) -> ScanWorker<W> {
    ScanWorker {
        table: NodeTable::default(),
        walk: W::Scratch::default(),
        scan: engine.state_for(programs),
        report: ScanReport::default(),
    }
}

/// Keeps the found rows of one file: every word cell is copied to owned text while the file's words are lent.
struct Keeping<'a> {
    /// The columns of each selected rule's rows.
    columns: &'a EmitColumns,
    /// The file being scanned.
    file: FileId,
    /// Where the worker keeps what it found.
    report: &'a mut ScanReport,
    /// The first target a row did not fit the columns of.
    misfit: Option<FoundTarget>,
}

impl FoundRows for Keeping<'_> {
    fn found(&mut self, target: FoundTarget, row: FoundCells<'_>, words: &FileWords) {
        let of_rule = |rule: SelectedRuleOrdinal| {
            self.columns
                .0
                .get(usize::try_from(rule.0).unwrap_or(usize::MAX))
        };
        let kinds = match target {
            FoundTarget::Rule(rule) => of_rule(rule).map(|columns| &columns.emit),
            FoundTarget::Frontier(rule, input) => {
                of_rule(rule).and_then(|columns| columns.frontier.get(input.0))
            }
        };
        let Some(kinds) = kinds.filter(|kinds| kinds.0.len() == row.0.len()) else {
            if self.misfit.is_none() {
                self.misfit = Some(target);
            }
            return;
        };
        let cells = row
            .0
            .iter()
            .zip(&kinds.0)
            .map(|(cell, kind)| match kind {
                CellKind::Symbol => KeptCell::Text(KeptText(text(words, *cell).0.into())),
                CellKind::Node
                | CellKind::File
                | CellKind::Span
                | CellKind::Number
                | CellKind::Flag => KeptCell::Number(*cell),
            })
            .collect();
        let file = self.file;
        match target {
            FoundTarget::Rule(rule) => self.report.found.push(ScanRow { file, rule, cells }),
            FoundTarget::Frontier(rule, input) => self.report.frontier.push(FrontierRow {
                file,
                rule,
                input,
                cells,
            }),
        }
    }
}

/// The visit that scans the file its language lends.
struct Visiting<'a> {
    /// The engine.
    engine: &'a dyn EnginePort,
    /// The rule set.
    programs: &'a Programs,
    /// The worker's per-file engine state.
    state: &'a mut ScanScratch,
    /// Where the found rows go.
    keeping: Keeping<'a>,
    /// What the scan answered; nothing until the language lends the file.
    answer: Option<Result<ScanCounters, ScanError>>,
}

impl FileVisit for Visiting<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        self.answer = Some(
            self.engine
                .scan(self.programs, file, self.state, &mut self.keeping),
        );
    }
}

/// One file, in the worker that took it: walked by `walk` into the worker's table, lent to the engine's scan, its found rows kept
/// in the worker's report. Rows found before a fault stay kept: the fault says whose rows of the file are not complete.
///
/// # Errors
///
/// `Walk` when the language does not read the file or its parser fails, `NotLent` when it lends no file, `Rule` when a rule's
/// program read a relation refused or filled in part, `Columns` when a found row does not fit the columns the run holds.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::errors::FileScanError;
/// use ryo_execution_runtime::application::file_scan::service::{scanned, worker};
/// use ryo_execution_runtime::application::file_scan::types::{EmitColumns, ScanRun};
/// use ryo_language::domain::files::types::SourcePath;
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_language::inbound::use_cases::walk::errors::WalkError;
/// use ryo_language::inbound::use_cases::walk::types::WalkRequest;
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
/// use ryo_relations::domain::relations::types::FileId;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let (programs, columns) = (Programs::default(), EmitColumns::default());
/// let mut worker = worker::<WalkUseCaseImpl>(&engine, &programs);
/// let file = WalkRequest { path: SourcePath("a.txt"), file: FileId(0), source: ByteSlice(b"") };
/// let run = ScanRun { programs: &programs, columns: &columns };
/// let answer = scanned(&WalkUseCaseImpl::new(), &engine, run, &mut worker, file);
/// assert_eq!(answer, Err(FileScanError::Walk { cause: WalkError::Unsupported }));
/// ```
pub fn scanned<W: WalkUseCase>(
    walk: &W,
    engine: &dyn EnginePort,
    run: ScanRun<'_>,
    worker: &mut ScanWorker<W>,
    file: WalkRequest<'_>,
) -> Result<ScanCounters, FileScanError> {
    let ScanWorker {
        table,
        walk: buffers,
        scan,
        report,
    } = worker;
    let mut visiting = Visiting {
        engine,
        programs: run.programs,
        state: scan,
        keeping: Keeping {
            columns: run.columns,
            file: file.file,
            report,
            misfit: None,
        },
        answer: None,
    };
    let walked = walk.walk(
        file,
        needs(run.programs),
        &mut builder(table, W::KIND_COUNT),
        buffers,
        &mut visiting,
    );
    match (walked, visiting.answer, visiting.keeping.misfit) {
        (Err(cause), _, _) => Err(FileScanError::Walk { cause }),
        (Ok(()), None, _) => Err(FileScanError::NotLent),
        (Ok(()), Some(Err(fault)), _) => Err(FileScanError::Rule { fault }),
        (Ok(()), Some(Ok(_)), Some(target)) => Err(FileScanError::Columns { target }),
        (Ok(()), Some(Ok(counted)), None) => Ok(counted),
    }
}

/// The files one worker took, each scanned in order: a faulted file is listed in the worker's report and the run goes on.
/// Answers what the scans counted together.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::service::{ran, worker};
/// use ryo_execution_runtime::application::file_scan::types::{EmitColumns, ScanRun};
/// use ryo_language::domain::files::types::SourcePath;
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_language::inbound::use_cases::walk::types::WalkRequest;
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
/// use ryo_relations::domain::relations::types::FileId;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let (programs, columns) = (Programs::default(), EmitColumns::default());
/// let mut worker = worker::<WalkUseCaseImpl>(&engine, &programs);
/// let file = WalkRequest { path: SourcePath("a.txt"), file: FileId(0), source: ByteSlice(b"") };
/// let run = ScanRun { programs: &programs, columns: &columns };
/// let _counted = ran(&WalkUseCaseImpl::new(), &engine, run, &mut worker, &[file]);
/// assert_eq!(worker.report.faults.len(), 1);
/// ```
pub fn ran<W: WalkUseCase>(
    walk: &W,
    engine: &dyn EnginePort,
    run: ScanRun<'_>,
    worker: &mut ScanWorker<W>,
    files: &[WalkRequest<'_>],
) -> ScanCounters {
    let mut total = ScanCounters::default();
    let sum = |one: InstructionCount, other: InstructionCount| {
        InstructionCount(one.0.saturating_add(other.0))
    };
    for file in files {
        match scanned(walk, engine, run, worker, *file) {
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

/// The run's report from its workers': the findings sorted by file, rule and cells; the frontier rows by file, rule and entry,
/// in the order each file handed them; the faulted files by file.
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

/// A report's findings as the harness compares them: per rule and file, each row as its text, sorted.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::service::harness_rows;
/// use ryo_execution_runtime::application::file_scan::types::ScanReport;
///
/// assert!(harness_rows(&ScanReport::default()).0.is_empty());
/// ```
#[must_use]
pub fn harness_rows(report: &ScanReport) -> HarnessRows {
    let mut rows = HarnessRows::default();
    for row in &report.found {
        let mut written = String::new();
        for (place, cell) in row.cells.iter().enumerate() {
            if place > 0 {
                written.push(CELL_SEPARATOR);
            }
            match cell {
                KeptCell::Text(word) => written.push_str(&word.0),
                KeptCell::Number(number) => written.push_str(&number.0.to_string()),
            }
        }
        let at = RuleFile {
            rule: row.rule,
            file: row.file,
        };
        rows.0.entry(at).or_default().push(RowText(written.into()));
    }
    for each in rows.0.values_mut() {
        each.sort_unstable();
    }
    rows
}

/// Compares the two engines' rows of the accepted rules, per rule and file; answers how many rules were compared.
///
/// # Errors
///
/// The first rule and file whose rows differ.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::service::compared;
/// use ryo_execution_runtime::application::file_scan::types::HarnessRows;
/// use ryo_relations::domain::relations::types::Count;
///
/// assert_eq!(compared(&HarnessRows::default(), &HarnessRows::default(), &[]), Ok(Count(0)));
/// ```
pub fn compared(
    old: &HarnessRows,
    new: &HarnessRows,
    accepted: &[SelectedRuleOrdinal],
) -> Result<Count, HarnessDifference> {
    let places: std::collections::BTreeSet<RuleFile> = old
        .0
        .keys()
        .chain(new.0.keys())
        .filter(|place| accepted.contains(&place.rule))
        .copied()
        .collect();
    for at in places {
        let (before, after) = (
            old.0.get(&at).cloned().unwrap_or_default(),
            new.0.get(&at).cloned().unwrap_or_default(),
        );
        if before != after {
            return Err(HarnessDifference {
                at,
                old: before,
                new: after,
            });
        }
    }
    Ok(Count(accepted.len()))
}

/// `both`'s comparison of a run on the build branch: the old engine's rows against the report's findings, for the rules the new
/// engine accepted, reported by count.
///
/// # Errors
///
/// `EngineHarness` naming the first rule and file whose rows differ.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::service::both;
/// use ryo_execution_runtime::application::file_scan::types::{HarnessRows, ScanReport};
///
/// assert!(both(&HarnessRows::default(), &ScanReport::default(), &[]).is_ok());
/// ```
pub fn both(
    old: &HarnessRows,
    report: &ScanReport,
    accepted: &[SelectedRuleOrdinal],
) -> Result<Count, OperatingError> {
    let compared = compared(old, &harness_rows(report), accepted).map_err(|difference| {
        OperatingError::EngineHarness {
            reason: engine_difference(&difference),
        }
    })?;
    engine_compared(compared.0);
    Ok(compared)
}
