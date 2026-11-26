//! Read only demanded numeric occurrences, preserving partiality and real source IDs.


use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::service::number;
use crate::domain::model::names::service::fixed_word;
use crate::domain::node::service::{node_cell, semantic_nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::{nodes::types::Node, syntax::{types::Kind}};
use ryo::languages;

/// Optional numeric literal reader; no projection runs until this relation is requested.
pub(crate) struct NumericLiteralItem;

// The engine fills `numeric-literal` from the walked file: every row of the file, once.
ryo::fills! {
    NumericLiteralItem => languages::NumericLiteral, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::NumericLiteral as languages::CatalogRelation>::Row;
        let memo = &mut file.model.words.get_mut().fixed;
        let value = &mut file.buffers.text;
        for Numbered { id, .. } in semantic_nodes_of_kind(&file.syntax, Kind::Node(Node::Constant)) {
            if let Some(found) = number(&file.syntax, id, value) {
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    kind: fixed_word(words, memo, found.kind),
                    value: word(words, languages::NameText(&value.0)),
                    exact_digits: found.exact,
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
    }
}
