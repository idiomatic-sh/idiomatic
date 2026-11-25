//! The `scope` relation, read from the HTML DOM.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{is_root, kind_bits, owner, section};
use crate::domain::syntax::types::{Kind, Rootness, Section};
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, span_at};
use crate::domain::walked_values::service as walked;
use ryo::languages;
/// The reader the `parse` contract hands the `scope` relation to, which reads its rows from a file's tree.
pub(crate) struct ScopeItem;
ryo::reads! {
 ScopeItem => languages::Scope |tree, at, _symbols| {
 kinds: [Kind::Node(Node::SourceFile), Kind::Node(Node::Element)] as kind_bits,
 require (is_root(tree, at) == Rootness::Root || section(tree, at) == crate::domain::syntax::types::Section::Section);
row { parent: node(owner(tree, at)), child: node(at), file: file, span: span(at) }
 }
}

// The engine fills `scope` from the walked file: the root and every section, each under the section around it.
ryo::fills! {
    ScopeItem => languages::Scope, walk(HtmlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Scope as languages::CatalogRelation>::Row;
        let view = *file;
        let _ = words;
        for at in elements(&view) {
            if matches!(view_kind(&view, at), Kind::Node(Node::SourceFile | Node::Element))
                && (walked::is_root(&view, at) == Rootness::Root || walked::section(&view, at) == Section::Section)
            {
                rows.write(Row {
                    parent: node_cell(&view, walked::owner(&view, at)),
                    child: node_cell(&view, at),
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
