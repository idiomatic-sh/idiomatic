//! The `comment` relation, read from the HTML DOM.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, property, span_at, word};
use crate::domain::syntax::types::Property;
use ryo::languages;
/// The item the engine's fill of the `comment` relation is registered on.
pub(crate) struct CommentItem;

// The engine fills `comment` from the walked file: every comment, once.
ryo::fills! {
    CommentItem => languages::Comment, walk(HtmlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Comment as languages::CatalogRelation>::Row;
        let view = *file;
        for at in elements(&view) {
            if view_kind(&view, at) == Kind::Node(Node::Comment) {
                rows.write(Row {
                    id: node_cell(&view, at),
                    text: word(words, property(&view, at, Property::Value)),
                    file: view.file,
                    span: span_at(&view, at),
                    doc: languages::Flag::Lowered,
                });
            }
        }
    }
}

