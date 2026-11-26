//! Optional decoded bytes; source spelling is never parsed by this reader.

use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::service::bytes_hex;
use crate::domain::model::view::service::built;
use crate::domain::model::view::types::ModelPart;
use crate::domain::node::service::{node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::{nodes::types::Node, syntax::{types::Kind}};
use ryo::languages::{self, NameText};
/// Optional reader for original byte-string occurrences.
pub(crate) struct BytesLiteralItem;

// The engine fills `bytes-literal` from the walked file: every row of the file, once.
ryo::fills! {
    BytesLiteralItem => languages::BytesLiteral, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::BytesLiteral as languages::CatalogRelation>::Row;
        // The owner of a node is the scope the model says it is evaluated in.
        built(file.model, &mut file.built, ModelPart::Lexical, file.syntax.tree);
        let owned = &file.model.lexical.scope_of;
        let hex = &mut file.buffers.text;
        for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(Node::Constant)) {
            if bytes_hex(node, hex).is_some() {
                rows.write(Row {
                    occurrence: node_cell(&file.syntax, id),
                    owner: node_cell(&file.syntax, owned.get(usize::from(id)).copied().unwrap_or(languages::Position(0))),
                    hex_value: word(words, NameText(&hex.0)),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
    }
}
