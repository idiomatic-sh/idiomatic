//! The `scope` relation, read from the HTML DOM.
use crate::domain::nodes::types::HtmlNode;
use crate::domain::syntax::service::{is_root, kind_bits, owner, section};
use crate::domain::syntax::types::{Kind, Rootness};
use ryo::languages;
/// The reader the `parse` contract hands the `scope` relation to, which reads its rows from a file's tree.
pub(crate) struct ScopeItem;
ryo::reads! {
 ScopeItem => languages::Scope |tree, at, _symbols| {
 kinds: [Kind::Node(HtmlNode::SourceFile), Kind::Node(HtmlNode::Element)] as kind_bits,
 require (is_root(tree, at) == Rootness::Root || section(tree, at) == crate::domain::syntax::types::Section::Section);
row { parent: node(owner(tree, at)), child: node(at), file: file, span: span(at) }
 }
}

#[cfg(test)]
#[path = "item_test.rs"]
mod tests;
