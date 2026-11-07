//! The `string_literal` relation of the language: every string value, as the library decoded it, in its place. A quoted key is a key,
//! not a string literal: `entry` reads it. A string whose escapes the library refuses has no value, and the relation says so.

use crate::domain::model::service::decoded;
use crate::domain::model::service::stringness;
use crate::domain::model::types::Model;
use crate::domain::model::types::Stringness;
use crate::domain::relations::constants::UNDECODED;
use crate::domain::syntax::service::{kind_bits, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `string_literal` relation to, which reads its rows from a file's tree.
pub(crate) struct StringLiteralItem;

ryo::reads! {
    StringLiteralItem => languages::StringLiteral, model(Model) |tree, model, at, symbols| {
        kinds: [
            Kind::Token(Token::BasicString),
            Kind::Token(Token::LiteralString),
            Kind::Token(Token::MlBasicString),
            Kind::Token(Token::MlLiteralString),
        ] as kind_bits,
        require (stringness(tree, at) == Stringness::Value);
        when decoded(tree, symbols, at) {
            Some(value) => {
                row {
                    occurrence: node(at),
                    owner: node(parent(tree, at)),
                    value: value(value),
                    file: file,
                    span: span(at),
                    import: value(languages::Flag::Lowered),
                }
            }
            None => {
                partial UNDECODED => languages::Missing::Any;
            }
        }
    }
}
