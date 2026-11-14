//! The `string_literal` relation of the language: a string literal with its escapes read, in its place; one in the arguments of a
//! standard macro whose arguments are expressions is read there too. The file a `#[path = "…"]` names is an import specifier.

use crate::domain::fact_extraction::service::specifier;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::{child, kind_bits};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `string_literal` relation's rows to.
pub(crate) struct StringLiteralItem;

ryo::reads! {
    StringLiteralItem => languages::StringLiteral, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Literal)] as kind_bits,
        expanded,
        given string = child(tree, at, Kind::Token(Token::String))?;
        given value = languages::intern_derived(symbols, tree.source, languages::decoded_at(tree.elements, string)?);
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            value: value(value),
            file: file,
            span: span(at),
            import: value(specifier(tree, at)),
        }
    }
}
