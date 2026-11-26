//! Read decoded literal elements from f-strings.


use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::service::fragments;
use crate::domain::node::service::{node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node as TypedNode;
use crate::domain::syntax::types::Kind as TypedKind;

use ryo::languages::{self};

/// The optional decoded f-string literal reader.
pub(crate) struct InterpolatedStringLiteralItem;

// The engine fills `interpolated-string-literal` from the walked file: every row of the file, once.
ryo::fills! {
    InterpolatedStringLiteralItem => languages::InterpolatedStringLiteral, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::InterpolatedStringLiteral as languages::CatalogRelation>::Row;
        let mut pieces = Vec::new();
        for Numbered { id, node } in nodes_of_kind(&file.syntax, TypedKind::Node(TypedNode::JoinedStr)) {
            pieces.clear();
            fragments(node, &mut pieces);
            for piece in &pieces {
                rows.write(Row {
                    id: node_cell(&file.syntax, piece.id),
                    owner: node_cell(&file.syntax, id),
                    value: word(words, piece.value),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, piece.id),
                });
            }
        }
    }
}
