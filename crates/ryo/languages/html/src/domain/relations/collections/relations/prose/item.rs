//! The `prose` relation, read from the HTML DOM.
use crate::domain::nodes::types::Node;
use crate::domain::runs::service::{run_name, run_owner, run_words};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::{Kind, Wording};
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, span_at, word};
use crate::domain::walked_values::service as walked;
use ryo::languages;
/// The reader the `parse` contract hands the `prose` relation to, which reads its rows from a file's tree.
pub(crate) struct ProseItem;
ryo::reads! {
 ProseItem => languages::Prose |tree, at, symbols| {
 kinds: [Kind::Node(Node::Element)] as kind_bits,
 require (run_words(tree, at) == Wording::Words);
row { id: node(at), owner: node(run_owner(tree, at)), text: value(run_name(tree, symbols, at)), file: file, span: span(at) }
 }
}

// The engine fills `prose` from the walked file: every block element that holds a run of words, with the run.
ryo::fills! {
    ProseItem => languages::Prose, walk(HtmlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Prose as languages::CatalogRelation>::Row;
        let view = *file;
        for at in elements(&view) {
            if view_kind(&view, at) == Kind::Node(Node::Element) && walked::run_words(&view, at) == Wording::Words {
                rows.write(Row {
                    id: node_cell(&view, at),
                    owner: node_cell(&view, walked::run_owner(&view, at)),
                    text: word(words, &walked::run_name(&view, at)),
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
