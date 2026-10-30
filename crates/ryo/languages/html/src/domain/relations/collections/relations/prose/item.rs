//! The `prose` relation, read from the HTML DOM.
use crate::domain::nodes::types::Node;
use crate::domain::runs::service::{run_name, run_owner, run_words};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::{Kind, Wording};
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

#[cfg(test)]
#[path = "item_test.rs"]
mod tests;
