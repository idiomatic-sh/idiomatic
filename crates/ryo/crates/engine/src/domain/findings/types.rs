//! The types of the `findings` concept: the rows a scan hands over, the frontier rows the project pass reads back, and what a scan
//! counted.

use ryo_language::domain::facts::types::{CellWord, FileWords};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// Where the host takes found rows as they are complete.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{FoundCells, FoundRows, FoundTarget};
/// use ryo_language::domain::facts::types::FileWords;
///
/// struct Counted(usize);
/// impl FoundRows for Counted {
///     fn found(&mut self, _target: FoundTarget, _row: FoundCells<'_>, _words: &FileWords) {
///         self.0 += 1;
///     }
/// }
/// let mut counted = Counted(0);
/// let target = FoundTarget::Rule(ryo_values::domain::rule_requests::types::SelectedRuleOrdinal(0));
/// counted.found(target, FoundCells(&[]), &FileWords::default());
/// assert_eq!(counted.0, 1);
/// ```
pub trait FoundRows {
    /// One found row of a rule, or of a frontier the project pass reads; its word cells mean something only with `words`, during
    /// this call: the host copies each word column to owned text (`facts::service::text`) before it keeps the row.
    fn found(&mut self, target: FoundTarget, row: FoundCells<'_>, words: &FileWords);
}

/// Whose row it is.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::FoundTarget;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// assert_ne!(FoundTarget::Rule(SelectedRuleOrdinal(0)), FoundTarget::Rule(SelectedRuleOrdinal(1)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundTarget {
    /// A finding of the rule.
    Rule(SelectedRuleOrdinal),
    /// A row of a cross-file rule's frontier, by its place in the rule's plan frontier.
    Frontier(SelectedRuleOrdinal, InputNumber),
}

/// The cells of one found row, in the emit's column order, lent for the call.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::FoundCells;
/// use ryo_language::domain::facts::types::CellWord;
///
/// assert_eq!(FoundCells(&[CellWord(3)]).0.len(), 1);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FoundCells<'a>(pub &'a [CellWord]);

/// What each column of a found row holds, in column order: a `Symbol` column holds a word of the file, which the host copies to
/// owned text; every other kind is a number.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::ColumnKinds;
/// use ryo_relations::domain::relations::types::CellKind;
///
/// assert_eq!(ColumnKinds(vec![CellKind::Number, CellKind::Symbol]).0.len(), 2);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ColumnKinds(pub Vec<CellKind>);

/// The columns of one accepted rule's rows, as its lowering emits them: of its findings, and of each entry of its plan's
/// frontier, by the entry's place.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::RuleColumns;
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

/// The frontier rows every file handed over, lent back by the host in file order.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{FrontierCells, FrontierSource};
/// use ryo_values::domain::plan_parts::types::InputNumber;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// struct NoFile;
/// impl FrontierSource for NoFile {
///     fn rows(&self, _rule: SelectedRuleOrdinal, _input: InputNumber, _each: &mut dyn FnMut(FrontierCells<'_>)) {}
/// }
/// let mut rows = 0;
/// NoFile.rows(SelectedRuleOrdinal(0), InputNumber(0), &mut |_| rows += 1);
/// assert_eq!(rows, 0);
/// ```
pub trait FrontierSource {
    /// Calls `each` with every row of a rule's frontier entry, in file order then hand-over order.
    fn rows(
        &self,
        rule: SelectedRuleOrdinal,
        input: InputNumber,
        each: &mut dyn FnMut(FrontierCells<'_>),
    );
}

/// One frontier row read back: owned text and numbers, never a word of a file.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{FrontierCell, FrontierCells};
/// use ryo_relations::domain::terms::types::NameText;
///
/// assert_eq!(FrontierCells(&[FrontierCell::Text(NameText("os"))]).0.len(), 1);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FrontierCells<'a>(pub &'a [FrontierCell<'a>]);

/// One cell of a frontier row.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::FrontierCell;
/// use ryo_language::domain::facts::types::CellWord;
///
/// assert_eq!(FrontierCell::Number(CellWord(4)), FrontierCell::Number(CellWord(4)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierCell<'a> {
    /// A text the file's row held.
    Text(NameText<'a>),
    /// A number, span bound or flag, as its stored bits.
    Number(CellWord),
}

/// What a scan did, for the run's measures.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{InstructionCount, ScanCounters};
///
/// assert_eq!(ScanCounters::default().steps, InstructionCount(0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScanCounters {
    /// Instructions executed.
    pub steps: InstructionCount,
    /// Anchor candidates entered.
    pub candidates: InstructionCount,
    /// Rows written into made tables.
    pub made_rows: InstructionCount,
}

/// A count of engine events.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::InstructionCount;
///
/// assert!(InstructionCount(1) > InstructionCount(0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstructionCount(pub u64);
