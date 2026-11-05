//! The types of the `finish` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. A finding is numbers: the row a query emitted, where it is, and which
//! rule it is of. Text is read back only when the report is written.

use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Value};
use ryo_relations::domain::terms::types::UnknownName;

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// A line of a file, counting from one; zero says the row has no place in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FileLine(pub u32);

/// A character of a line, counting from one; zero says the row has no place in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ColumnNumber(pub u32);

/// Where the lines of a file start, in bytes, counting the first as zero.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LineStarts(pub Vec<u32>);

/// A place in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Point {
    /// The line.
    pub line: FileLine,
    /// The character in the line.
    pub column: ColumnNumber,
}

/// One row a query emitted, with its place: the prototype's `Found`, with the whole row for a report
/// that shows more than the place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FoundRow {
    /// The rule that emitted it.
    pub rule: RuleNumber,
    /// The file it was read from, or the file its subject names.
    pub file: FileId,
    /// Where in the file it is.
    pub place: Point,
    /// The emitted row, every name as the place of its text.
    pub row: Vec<Value>,
}

/// Where the text behind a report's numbers is: the paths of the files, the text the workers made,
/// and the names the cross-file steps made.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Resolving<'a> {
    /// The path of each file, by file number.
    pub paths: &'a [FilePath],
    /// The text each worker made, by worker number.
    pub arenas: Arenas<'a>,
    /// The names the cross-file steps made.
    pub cross: &'a Symbols,
    /// What the reasons of the languages linked are called.
    pub unknown: &'a [UnknownName],
}

/// Which file the bytes held are of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum HeldFile {
    /// None yet.
    #[default]
    Nothing,
    /// This file.
    File(FileId),
}

/// The text each worker made, by worker number, borrowed.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Arenas<'a>(pub &'a [ArenaBytes]);

/// The bytes of the file read last, kept so a report that shows many names of one file reads it once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Loaded {
    /// Which file the bytes are of.
    pub held: HeldFile,
    /// The bytes.
    pub bytes: FileBytes,
}

use ryo_values::domain::runs::types::RuleNumber;
