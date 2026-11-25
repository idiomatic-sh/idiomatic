//! The `comment` relation, read from the HTML DOM.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, value};
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, property, span_at, word};
use crate::domain::syntax::types::Property;
use ryo::languages;
/// The reader the `parse` contract hands the `comment` relation to, which reads its rows from a file's tree.
pub(crate) struct CommentItem;
ryo::reads! {
 CommentItem => languages::Comment |tree, at, symbols| {
 kinds: [Kind::Node(Node::Comment)] as kind_bits,
 row { id: node(at), text: value(value(tree, symbols, at)), file: file, span: span(at), doc: value(languages::Flag::Lowered) }
 }
}

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

