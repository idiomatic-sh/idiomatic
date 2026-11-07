//! The types of the `stored_findings` concept: what a store keeps of a finding: where it was made, its values and the text it names. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::descriptions::types::ColumnName;
use crate::domain::records::types::Unreadable;
use crate::domain::row_texts::types::{Cell, PlacedColumn, PlacedLine};
use crate::domain::runs::types::RuleId;
use ryo_language::domain::files::types::FilePath;

/// A finding a run showed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFinding {
    /// The rule that found it.
    pub rule: RuleId,
    /// Which visit made it.
    pub made: FindingMade,
    /// Where it is in its subject.
    pub place: FindingPlace,
    /// What it is about.
    pub subject: StoredCell,
    /// What tells it apart from the rule's others: the value of each of the rule's key columns, in order.
    pub key: Vec<StoredCell>,
    /// Its columns, in the order the rule writes them.
    pub values: Vec<StoredValue>,
}

/// Where in its file a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredPlace {
    /// The line.
    pub line: PlacedLine,
    /// The character in the line.
    pub column: PlacedColumn,
}

/// One column of a finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredValue {
    /// Its heading.
    pub column: ColumnName,
    /// Its value.
    pub value: StoredCell,
}

/// A cell of a finding as the store keeps it: its value, or nothing when the value reads as nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoredCell {
    /// The value.
    Value(Cell),
    /// The value reads as nothing.
    Nothing,
}

impl From<Option<Cell>> for StoredCell {
    fn from(cell: Option<Cell>) -> Self {
        cell.map_or(Self::Nothing, Self::Value)
    }
}

/// A file no language read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredUnread {
    /// The file.
    pub file: StoredFileNumber,
    /// Why: no linked language reads such a file, or what the language said when it failed.
    pub reason: Unreadable,
}

/// Something a rule could not decide, and the file that is the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredUnknown {
    /// The rule.
    pub rule: RuleId,
    /// The file whose relation is read only in part.
    pub path: FilePath,
    /// What is partial there and why.
    pub message: UnknownText,
}

/// What a rule could not decide, as shown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnknownText(pub Box<str>);

/// Which visit made a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingMade {
    /// The visit of the file at this place among the stored files.
    InFile(StoredFileNumber),
    /// The rules that read across files, once every file was visited.
    AcrossFiles,
}

/// Where in its subject a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingPlace {
    /// At this line and character.
    At(StoredPlace),
    /// Nowhere in particular: the finding is about its subject as a whole.
    Unplaced,
}

/// A file, by its place among the stored files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoredFileNumber(pub u32);
