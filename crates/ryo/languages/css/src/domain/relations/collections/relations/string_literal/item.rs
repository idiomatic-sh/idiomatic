//! The `string_literal` relation of the language: every string-token, as the library decoded it (its quotes and escapes resolved),
//! owned by the declaration or rule it is written in. A url-token is no string: `reference` reads the path it points at. The string
//! an `@import` names its stylesheet by is an import specifier.

use crate::domain::syntax::service::{decoded, kind_bits};
use crate::domain::syntax::types::Kind;
use crate::domain::targets::service::{owner, specifier};
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `string_literal` relation to, which reads its rows from a file's tree.
pub(crate) struct StringLiteralItem;

ryo::reads! {
    StringLiteralItem => languages::StringLiteral |tree, at, symbols| {
        kinds: [Kind::Token(Token::String)] as kind_bits,
        given value = decoded(tree, symbols, at)?;
        row {
            occurrence: node(at),
            owner: node(owner(tree, at)),
            value: value(value),
            file: file,
            span: span(at),
            import: value(specifier(tree, at)),
        }
    }
}
