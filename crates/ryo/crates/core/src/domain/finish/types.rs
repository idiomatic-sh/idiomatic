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

use ryo_values::domain::runs::types::RuleNumber;
