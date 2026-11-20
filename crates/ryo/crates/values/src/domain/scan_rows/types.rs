//! The types of the `scan_rows` concept: plain data, public fields and derives, with no constructor and no inherent impl. A row the
//! scan of a file found, kept: its word cells as owned text, every other cell as its number, and where in the file it is.

use crate::domain::plan_parts::types::InputNumber;
use crate::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{FileId, Span};

/// The text a word of a file stood for, owned: it outlives the file's words.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::scan_rows::types::KeptText;
///
/// assert_eq!(&*KeptText("os".into()).0, "os");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeptText(pub Box<str>);

/// One cell of a kept row: owned text, or a number (a node, a file, a span, a count, a flag) as its bits. Never a word of a file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::CellWord;
/// use ryo_values::domain::scan_rows::types::{KeptCell, KeptText};
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

/// Where in its file a kept row is: the stretch of the first of its key columns, then its subject, that is a stretch of the file
/// or a node of it. A row with no place sorts before the placed ones.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::{Offset, Span};
/// use ryo_values::domain::scan_rows::types::RowPlace;
///
/// assert!(RowPlace::Unplaced < RowPlace::At(Span { start: Offset(0), end: Offset(0) }));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowPlace {
    /// The row has no place in its file.
    #[default]
    Unplaced,
    /// The row is this stretch of its file.
    At(Span),
}

/// One finding a rule found in a file, kept: ordered by file, then rule, then where it starts and ends, then cells.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::FileId;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
/// use ryo_values::domain::scan_rows::types::{RowPlace, ScanRow};
///
/// let row = |file, rule| ScanRow { file: FileId(file), rule: SelectedRuleOrdinal(rule), place: RowPlace::Unplaced, cells: Vec::new() };
/// assert!(row(0, 1) < row(1, 0));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScanRow {
    /// The file.
    pub file: FileId,
    /// The rule.
    pub rule: SelectedRuleOrdinal,
    /// Where in the file the row is.
    pub place: RowPlace,
    /// The row's cells, in the emit's column order.
    pub cells: Vec<KeptCell>,
}

/// One row a file hands a cross-file rule's frontier entry, kept for the project pass.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::FileId;
/// use ryo_values::domain::plan_parts::types::InputNumber;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
/// use ryo_values::domain::scan_rows::types::FrontierRow;
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

/// Why a file's scan is a fault, as the sentence that says so.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::scan_rows::types::FaultSentence;
///
/// assert_eq!(&*FaultSentence("file walked and not lent".into()).0, "file walked and not lent");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FaultSentence(pub Box<str>);

/// Whose fault a faulted scan is.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
/// use ryo_values::domain::scan_rows::types::FaultOf;
///
/// assert_ne!(FaultOf::File, FaultOf::Rule(SelectedRuleOrdinal(0)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultOf {
    /// No linked language walks files like this one.
    NoLanguage,
    /// The file's: a language walks files like it and failed on this one.
    File,
    /// A rule's: its rows of the file are not complete.
    Rule(SelectedRuleOrdinal),
}

/// A file's scan that faulted: whose fault it is, and the sentence that says why.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::scan_rows::types::{FaultOf, FaultSentence, ScanFault};
///
/// let fault = ScanFault { of: FaultOf::File, sentence: FaultSentence("file walked and not lent".into()) };
/// assert_eq!(fault.of, FaultOf::File);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanFault {
    /// Whose fault it is.
    pub of: FaultOf,
    /// Why.
    pub sentence: FaultSentence,
}
