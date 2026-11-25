//! The `reference` relation, read from the HTML DOM: every URL-valued attribute, with what it points at resolved to a path of the run
//! as a link is (`href="../guide.html#setup"` in `docs/a.html` points at `guide.html#setup`).
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, owner, reference, target};
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, span_at, target as view_target, word};
use crate::domain::walked_values::service as walked;
use crate::domain::syntax::types::Reference;
use ryo::languages;
/// The reader the `parse` contract hands the `reference` relation to, which reads its rows from a file's tree.
pub(crate) struct ReferenceItem;
ryo::reads! {
 ReferenceItem => languages::Reference |tree, at, symbols| {
 kinds: [Kind::Node(Node::Attribute)] as kind_bits,
 require (reference(tree, at) == crate::domain::syntax::types::Reference::Url);
row { occurrence: node(at), owner: node(owner(tree, at)), target: value(target(tree, symbols, at)), file: file, span: span(at) }
 }
}

// The engine fills `reference` from the walked file: every URL-valued attribute, with the path of the run it points at.
ryo::fills! {
    ReferenceItem => languages::Reference, walk(HtmlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Reference as languages::CatalogRelation>::Row;
        let view = *file;
        for at in elements(&view) {
            if view_kind(&view, at) == Kind::Node(Node::Attribute) && walked::reference(&view, at) == Reference::Url {
                rows.write(Row {
                    occurrence: node_cell(&view, at),
                    owner: node_cell(&view, walked::owner(&view, at)),
                    target: word(words, view_target(&view, at)),
                    file: view.file,
                    span: span_at(&view, at),
                });
            }
        }
    }
}

#[cfg(test)]
#[path = "item_test.rs"]
mod tests;
