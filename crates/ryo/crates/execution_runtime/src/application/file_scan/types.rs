//! The types of the `file_scan` operation: a worker's state, the columns a found row is read by, the rows kept as owned text and
//! numbers, the run's report, and the harness's comparison of two engines' rows.

use super::errors::FileScanError;
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::scratch::types::ScanScratch;
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::inbound::use_cases::walk::interface::WalkUseCase;
use ryo_relations::domain::relations::types::{CellKind, FileId};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// What each column of a found row holds, in column order: a `Symbol` column holds a word of the file, copied to owned text.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::ColumnKinds;
/// use ryo_relations::domain::relations::types::CellKind;
///
/// assert_eq!(ColumnKinds(vec![CellKind::Number, CellKind::Symbol]).0.len(), 2);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ColumnKinds(pub Vec<CellKind>);

/// The columns of one selected rule's rows: of its emit, and of each entry of its plan's frontier, by the entry's place.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::RuleColumns;
///
/// assert!(RuleColumns::default().frontier.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleColumns {
    /// The columns of the rule's findings.
    pub emit: ColumnKinds,
    /// The columns of each frontier entry.
    pub frontier: Vec<ColumnKinds>,
}

/// The columns of every selected rule, by rule ordinal: read from the plans the host holds, once for the run.
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

/// The text a word of a file stood for, owned: it outlives the file's words.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::KeptText;
///
/// assert_eq!(&*KeptText("os".into()).0, "os");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeptText(pub Box<str>);

/// One cell of a kept row: owned text, or a number (a node, a span bound, a count, a flag) as its bits. Never a word of a file.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::{KeptCell, KeptText};
/// use ryo_language::domain::facts::types::CellWord;
///
/// assert!(KeptCell::Text(KeptText("os".into())) < KeptCell::Number(CellWord(0)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeptCell {
    /// The text a word stood for while its file was scanned.
    Text(KeptText),
    /// A number.
    Number(CellWord),
}

/// One finding a rule found in a file, kept: ordered by file, then rule, then cells.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::ScanRow;
/// use ryo_relations::domain::relations::types::FileId;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let row = |file, rule| ScanRow { file: FileId(file), rule: SelectedRuleOrdinal(rule), cells: Vec::new() };
/// assert!(row(0, 1) < row(1, 0));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScanRow {
    /// The file.
    pub file: FileId,
    /// The rule.
    pub rule: SelectedRuleOrdinal,
    /// The row's cells, in the emit's column order.
    pub cells: Vec<KeptCell>,
}

/// One row a file hands a cross-file rule's frontier entry, kept for the project pass.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::FrontierRow;
/// use ryo_relations::domain::relations::types::FileId;
/// use ryo_values::domain::plan_parts::types::InputNumber;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let row = FrontierRow { file: FileId(0), rule: SelectedRuleOrdinal(0), input: InputNumber(0), cells: Vec::new() };
/// assert_eq!(row.input, InputNumber(0));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontierRow {
    /// The file.
    pub file: FileId,
    /// The rule.
    pub rule: SelectedRuleOrdinal,
    /// The frontier entry, by its place in the rule's plan frontier.
    pub input: InputNumber,
    /// The row's cells.
    pub cells: Vec<KeptCell>,
}

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

/// One worker's state: the node table, the language's buffers and the engine's per-file state, each cleared per file and never
/// freed, and what the worker found. Made by `service::worker`.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::service::worker;
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// assert!(worker::<WalkUseCaseImpl>(&engine, &Programs::default()).report.found.is_empty());
/// ```
pub struct ScanWorker<W: WalkUseCase> {
    /// The table of the file being scanned.
    pub table: NodeTable,
    /// The language's per-worker buffers.
    pub walk: W::Scratch,
    /// The engine's per-file state.
    pub scan: ScanScratch,
    /// What the worker found in the files it scanned.
    pub report: ScanReport,
}

impl<W: WalkUseCase> std::fmt::Debug for ScanWorker<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// What every file of a run is scanned with: the rule set and the columns of its rules' rows.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::types::{EmitColumns, ScanRun};
///
/// let (programs, columns) = (Programs::default(), EmitColumns::default());
/// let run = ScanRun { programs: &programs, columns: &columns };
/// assert!(run.columns.0.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ScanRun<'a> {
    /// The rule set.
    pub programs: &'a Programs,
    /// The columns of each selected rule's rows.
    pub columns: &'a EmitColumns,
}

/// One engine's rows of the compared rules: per rule and file, each row as its owned text, sorted.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::HarnessRows;
///
/// assert!(HarnessRows::default().0.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HarnessRows(pub std::collections::BTreeMap<RuleFile, Vec<RowText>>);

/// A rule and a file.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::RuleFile;
/// use ryo_relations::domain::relations::types::FileId;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// assert!(RuleFile { rule: SelectedRuleOrdinal(0), file: FileId(1) } < RuleFile { rule: SelectedRuleOrdinal(1), file: FileId(0) });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuleFile {
    /// The rule.
    pub rule: SelectedRuleOrdinal,
    /// The file.
    pub file: FileId,
}

/// A row as owned text: its cells, each a word's text or a number, joined by tabs.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::RowText;
///
/// assert_eq!(&*RowText("3\t9\tos".into()).0, "3\t9\tos");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowText(pub Box<str>);

/// The first place the two engines' rows differ.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::{HarnessDifference, RowText, RuleFile};
/// use ryo_relations::domain::relations::types::FileId;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let at = RuleFile { rule: SelectedRuleOrdinal(0), file: FileId(1) };
/// let difference = HarnessDifference { at, old: vec![RowText("a".into())], new: Vec::new() };
/// assert_eq!(difference.old.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HarnessDifference {
    /// The rule and the file.
    pub at: RuleFile,
    /// The old engine's rows there.
    pub old: Vec<RowText>,
    /// The new engine's rows there.
    pub new: Vec<RowText>,
}
