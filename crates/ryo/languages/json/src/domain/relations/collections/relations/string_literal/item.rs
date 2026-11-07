//! The `string_literal` relation of the language: every string value, as the library decoded it, in its place. The name of a member is
//! a key, not a string literal: `entry` reads it.

use crate::domain::model::types::Model;
use crate::domain::relations::constants::UNDECODED;
use crate::domain::syntax::service::{decoded, keyness, kind_bits, parent};
use crate::domain::syntax::types::{Keyness, Kind};
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `string_literal` relation to, which reads its rows from a file's tree.
pub(crate) struct StringLiteralItem;

ryo::reads! {
    StringLiteralItem => languages::StringLiteral, model(Model) |tree, model, at, symbols| {
        kinds: [Kind::Token(Token::String)] as kind_bits,
        require (keyness(tree, at) == Keyness::Other);
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
