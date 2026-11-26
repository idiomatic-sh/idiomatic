//! The `sequence_element` relation of the language: every value of an array, and every member of an array of tables, in its place:
//! the two spellings of a sequence give the same rows.

use crate::domain::file_view::service::{modelled, node_cell, span_at};
use crate::domain::file_view::types::TomlWalk;
use crate::domain::walked_values::service as walked;
use ryo::languages;

/// The reader the `parse` contract hands the `sequence_element` relation to, which reads its rows from a file's tree.
pub(crate) struct SequenceElementItem;


// The engine fills `sequence_element` from the walked file: every value of an array and every member of an array of tables.
ryo::fills! {
    SequenceElementItem => languages::SequenceElement, walk(TomlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::SequenceElement as languages::CatalogRelation>::Row;
        let (model, syntax) = modelled(file);
        for fact in &model.elements {
            let at = Some(fact.occurrence);
            rows.write(Row {
                decoded: walked::said(&syntax, words, at),
                kind: fact.kind,
                occurrence: node_cell(&syntax, fact.occurrence),
                ordinal: fact.ordinal,
                owner: node_cell(&syntax, fact.owner),
                file: syntax.file,
                span: span_at(&syntax, fact.occurrence),
                text: walked::spanned(&syntax, words, fact.occurrence),
                boolean: walked::boolean(&syntax, at),
                integer: walked::integer(&syntax, at),
                float: walked::decoded_as(&syntax, words, at, languages::ValueKind::Float),
                datetime: walked::decoded_as(&syntax, words, at, languages::ValueKind::Datetime),
                radix: walked::radix(&syntax, at),
            });
        }
    }
}
