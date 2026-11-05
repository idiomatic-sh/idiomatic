//! The `comment` relation, read from the HTML DOM.
use crate::domain::nodes::types::HtmlNode;
use crate::domain::syntax::service::{kind_bits, value};
use crate::domain::syntax::types::Kind;
use ryo::languages;
/// The reader the `parse` contract hands the `comment` relation to, which reads its rows from a file's tree.
pub(crate) struct CommentItem;
ryo::reads! {
 CommentItem => languages::Comment |tree, at, symbols| {
 kinds: [Kind::Node(HtmlNode::Comment)] as kind_bits,
 row { id: node(at), text: value(value(tree, symbols, at)), file: file, span: span(at), doc: value(languages::Flag::Lowered) }
 }
}

#[cfg(test)]
#[path = "item_test.rs"]
mod tests;
