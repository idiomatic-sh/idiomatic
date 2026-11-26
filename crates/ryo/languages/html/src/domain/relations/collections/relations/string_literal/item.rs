//! The `string_literal` relation, read from the HTML DOM.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, parent as view_parent, property, span_at, word};
use crate::domain::syntax::types::Property;
use ryo::languages;
/// The item the engine's fill of the `string_literal` relation is registered on.
pub(crate) struct StringLiteralItem;

// The engine fills `string_literal` from the walked file: every attribute, with its decoded value.
ryo::fills! {
    StringLiteralItem => languages::StringLiteral, walk(HtmlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StringLiteral as languages::CatalogRelation>::Row;
        let view = *file;
        for at in elements(&view) {
            if view_kind(&view, at) == Kind::Node(Node::Attribute) {
                rows.write(Row {
                    occurrence: node_cell(&view, at),
                    owner: node_cell(&view, view_parent(&view, at)),
                    value: word(words, property(&view, at, Property::Value)),
                    file: view.file,
                    span: span_at(&view, at),
                    import: languages::Flag::Lowered,
                });
            }
        }
    }
}

