//! The `string_literal` relation, read from the HTML DOM.
use crate::domain::nodes::types::HtmlNode;
use crate::domain::syntax::service::{kind_bits, parent, value};
use crate::domain::syntax::types::Kind;
use ryo::languages;
/// The reader the `parse` contract hands the `string_literal` relation to, which reads its rows from a file's tree.
pub(crate) struct StringLiteralItem;
ryo::reads! {
 StringLiteralItem => languages::StringLiteral |tree, at, symbols| {
 kinds: [Kind::Node(HtmlNode::Attribute)] as kind_bits,
 row { occurrence: node(at), owner: node(parent(tree, at)), value: value(value(tree, symbols, at)), file: file, span: span(at), import: value(languages::Flag::Lowered) }
 }
}

#[cfg(test)]
#[path = "item_test.rs"]
mod tests;
