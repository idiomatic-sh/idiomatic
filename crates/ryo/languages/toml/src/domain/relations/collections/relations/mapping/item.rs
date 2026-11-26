//! The `mapping` relation of the language: every logical table of a document, once, by the element that first opens it: the root
//! for the document's own table, a header, an array table member, an inline table, or the part of a key that implies it.

use crate::domain::file_view::service::{modelled, node_cell};
use crate::domain::file_view::types::TomlWalk;
use ryo::languages;

/// The reader the `parse` contract hands the `mapping` relation to, which reads its rows from a file's tree.
pub(crate) struct MappingItem;


// The engine fills `mapping` from the walked file: every logical table, once, by the element that first opens it.
ryo::fills! {
    MappingItem => languages::Mapping, walk(TomlWalk) |file, _words, rows| {
        /// One row of the relation.
        type Row = <languages::Mapping as languages::CatalogRelation>::Row;
        let (model, syntax) = modelled(file);
        for fact in &model.mappings {
            rows.write(Row {
                id: node_cell(&syntax, fact.id),
                kind: fact.kind,
                ordinal: fact.ordinal,
                owner: node_cell(&syntax, fact.owner),
                file: syntax.file,
            });
        }
    }
}
