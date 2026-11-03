//! The types of the `value_cells` concept: the file a row's values are read in. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::FileId;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::terms::types::UnknownName;

/// The file the rows are of, and what turns its numbers into names and paths.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scene<'a> {
    /// The file.
    pub file: FileId,
    /// Its names.
    pub symbols: &'a Symbols,
    /// Its tree, which says where a node starts.
    pub elements: &'a Elements,
    /// Its bytes.
    pub source: SourceView<'a>,
    /// The path of each file, by file number.
    pub paths: &'a [FilePath],
    /// What the reasons of the languages linked are called.
    pub unknown: &'a [UnknownName],
}
