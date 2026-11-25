//! The `string_literal` relation, read from the HTML DOM.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, parent, value};
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, parent as view_parent, property, span_at, word};
use crate::domain::syntax::types::Property;
use ryo::languages;
/// The reader the `parse` contract hands the `string_literal` relation to, which reads its rows from a file's tree.
pub(crate) struct StringLiteralItem;
ryo::reads! {
 StringLiteralItem => languages::StringLiteral |tree, at, symbols| {
 kinds: [Kind::Node(Node::Attribute)] as kind_bits,
 row { occurrence: node(at), owner: node(parent(tree, at)), value: value(value(tree, symbols, at)), file: file, span: span(at), import: value(languages::Flag::Lowered) }
 }
}

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

#[cfg(test)]
#[path = "item_test.rs"]
mod tests;
