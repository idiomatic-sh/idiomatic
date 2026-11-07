//! The `reference` relation, read from the HTML DOM: every URL-valued attribute, with what it points at resolved to a path of the run
//! as a link is (`href="../guide.html#setup"` in `docs/a.html` points at `guide.html#setup`).
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, owner, reference, target};
use crate::domain::syntax::types::Kind;
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

#[cfg(test)]
#[path = "item_test.rs"]
mod tests;
