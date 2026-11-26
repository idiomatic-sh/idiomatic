//! The Python language's private file-scope relation (annotation evaluation), answered from a walked file while its view is live: the walk's
//! `named` item hands the request here; and the project notes of a walked file (its imports, read in source order) are made here.

use crate::domain::file_view::types::{PythonFile, PythonWalk};
use crate::domain::node::service::nodes_of_kind;
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::inbound::contracts::walk::contract::WalkPorts;
use ryo::languages::{
    self, FactAnswer, FactSource as _, FileWords, FilledColumns, NodeTable, Relation, Wanted, fillers,
};

/// Whether `text` is a name as this language spells one (a Python identifier: a letter or `_`, then letters, digits or
/// `_`; Unicode letters and digits count).
#[must_use]
pub(crate) fn is_identifier(text: &str) -> bool {
    let mut cs = text.chars();
    cs.next().is_some_and(|c| c.is_alphabetic() || c == '_') && cs.all(|c| c.is_alphanumeric() || c == '_')
}

/// One walked file, lent to the private relations.
pub(crate) struct NamedFile<'a, 'f> {
    /// The node table.
    pub(crate) table: &'a NodeTable,
    pub(crate) file: &'a mut PythonFile<'f>,
    pub(crate) words: &'a mut FileWords,
}

mod imports;
mod schemas;
mod named_semantics;

fn qualify_cells(
    schema: &languages::FileRelation,
    file: languages::FileId,
    columns: Vec<Vec<u64>>,
) -> Vec<languages::FactColumn> {
    columns
        .into_iter()
        .zip(schema.columns)
        .map(|(mut cells, (_, kind))| {
            if *kind == languages::CellKind::Node {
                for cell in &mut cells {
                    if *cell != u64::MAX {
                        *cell |= u64::from(file.0) << 32;
                    }
                }
            }
            languages::FactColumn(cells)
        })
        .collect()
}

impl NamedFile<'_, '_> {
    /// The project notes of a walked file: its imports, read in source order.
    pub(crate) fn noted(file: &mut PythonFile<'_>, table: &NodeTable, source: languages::ByteSlice<'_>) -> crate::domain::fact_notes::types::Note {
        let walked = NamedFile {
            table,
            file,
            words: &mut FileWords::default(),
        };
        crate::domain::fact_notes::service::noted_walk(&walked, source)
    }

    /// The rows of `relation` as the file's fillers write them: what a private relation is built from.
    pub(crate) fn fill(&mut self, relation: Relation, wanted: Wanted, words: &mut FileWords, out: &mut FilledColumns) -> FactAnswer {
        let mut source = fillers::file_source::<PythonWalk<WalkPorts>>(self.file);
        source.fill(relation, wanted, words, out)
    }
}

