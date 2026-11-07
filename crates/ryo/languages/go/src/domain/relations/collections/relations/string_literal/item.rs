//! The `string_literal` relation of the language: every string, interpreted or raw, with the text its escapes and its quotes stand
//! for. An import path and a struct tag are strings; an import path is marked as an import specifier. One whose escapes make a byte
//! that is no UTF-8 has the unknown value of a string that is no text.

use crate::domain::names::service::{specifier, string_or_unknown, string_token};
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `string_literal` relation to, which reads its rows from a file's tree.
pub(crate) struct StringLiteralItem;

ryo::reads! {
    StringLiteralItem => languages::StringLiteral |tree, at, symbols| {
        kinds: [Kind::Node(Node::BasicLit)] as kind_bits,
        require string_token(tree, at).is_some();
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            value: value(string_or_unknown(tree, symbols, at)),
            file: file,
            span: span(at),
            import: value(specifier(tree, at)),
        }
    }
}
