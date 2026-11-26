//! What the Python language reads of a walked file beside its fills: the spelling of a name, and the project notes of the file (its
//! imports, read in source order), which the project's namespace membership is read from.

use crate::domain::file_view::types::PythonFile;
use crate::domain::node::service::nodes_of_kind;
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, NodeTable};

/// Whether `text` is a name as this language spells one (a Python identifier: a letter or `_`, then letters, digits or
/// `_`; Unicode letters and digits count).
#[must_use]
pub(crate) fn is_identifier(text: &str) -> bool {
    let mut cs = text.chars();
    cs.next().is_some_and(|c| c.is_alphabetic() || c == '_') && cs.all(|c| c.is_alphanumeric() || c == '_')
}

/// One walked file, lent to the reader of its imports.
pub(crate) struct NamedFile<'a, 'f> {
    /// The node table.
    pub(crate) table: &'a NodeTable,
    pub(crate) file: &'a mut PythonFile<'f>,
}

mod imports;

impl NamedFile<'_, '_> {
    /// The project notes of a walked file: its imports, read in source order.
    pub(crate) fn noted(file: &mut PythonFile<'_>, table: &NodeTable, source: languages::ByteSlice<'_>) -> crate::domain::fact_notes::types::Note {
        let walked = NamedFile { table, file };
        crate::domain::fact_notes::service::noted_walk(&walked, source)
    }
}
