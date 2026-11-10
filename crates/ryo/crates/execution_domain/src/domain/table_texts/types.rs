//! The types of the `table_texts` concept: what a cell reads as when it is compared. Plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::FlagState;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::WholeNumber;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// A query's regular-expression pattern, keyed by its exact source text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegexPattern(pub String);

impl std::borrow::Borrow<str> for RegexPattern {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// What a cell reads as when it is compared: numbers as numbers, names and files as their text.
/// The text is borrowed from where it lives, so reading a cell allocates nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Datum<'a> {
    /// No source location exists for this cell.
    Unlocated,
    /// A whole number.
    Number(WholeNumber),
    /// A yes or no.
    Flag(FlagState),
    /// Text.
    Text(TextView<'a>),
    /// A list or a set of numbers.
    Numbers(&'a [WholeNumber]),
    /// A list or a set of text.
    Texts(&'a [LiteralText]),
}

/// Text read from where it lives, borrowed: the text of a cell as it is compared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextView<'a>(pub &'a str);

/// Where the text of the names and files in a file's rows is, while the rows are worked on: the
/// file's names and bytes, and the paths of the files.
#[derive(Clone, Copy, Debug)]
pub struct Texts<'a> {
    /// The names of the file.
    pub symbols: &'a Symbols,
    /// The bytes of the file.
    pub source: SourceView<'a>,
    /// The path of each file of the run, by file number.
    pub paths: &'a [FilePath],
    /// The text each worker made, by worker number: where a name that has left its file keeps its text.
    pub arenas: &'a [ArenaBytes],
}

use ryo_values::domain::files::types::SourceView;
