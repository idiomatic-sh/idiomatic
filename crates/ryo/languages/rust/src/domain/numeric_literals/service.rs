//! The `numeric_literals` service: free functions named by verb that read what a literal node holds of a number, with no IO and no
//! ports.

use super::constants::{NUMERIC_KIND, NUMERIC_VALUE};
use super::types::LiteralNumber;
use crate::domain::syntax::service::child;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Position, Tree};

/// What the literal node at `at` holds of a number: its kind and value as the adapter stored them, that it is a number with none, or
/// that it is no number.
pub(crate) fn stored<'a>(tree: &Tree<'a>, at: Position) -> LiteralNumber<'a> {
    let number = [Token::IntNumber, Token::FloatNumber]
        .into_iter()
        .any(|token| child(tree, at, Kind::Token(token)).is_some());
    if !number {
        return LiteralNumber::NotNumeric;
    }
    match (
        languages::property_at(tree.elements, at, NUMERIC_KIND),
        languages::property_at(tree.elements, at, NUMERIC_VALUE),
    ) {
        (Some(kind), Some(value)) => LiteralNumber::Known { kind, value },
        (Some(_) | None, _) => LiteralNumber::Unread,
    }
}
