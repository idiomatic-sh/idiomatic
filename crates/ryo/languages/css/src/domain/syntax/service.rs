//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no ports. It maps a kind to the number
//! the node table holds and back, says its name, and says the role a node of a kind has under its parent.

use ryo::languages;

use super::constants::{BANG, IMPORTANT};
use super::types::{Field, Kind, Word};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use strum::IntoEnumIterator as _;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

/// The number the element table holds for a role.
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "a role is a field-less enum with a u16 representation: its number is the cast"
)]
pub(crate) const fn field_bits(field: Field) -> languages::FieldId {
    languages::FieldId(field as u16)
}

/// A role's name, in kebab case: text that leaves the engine.
#[must_use]
pub(crate) fn field_name(field: Field) -> languages::NameText<'static> {
    languages::NameText(field.into())
}

/// Every role, in the order of their numbers.
pub(crate) fn fields() -> impl Iterator<Item = Field> {
    Field::iter()
}

/// The role of the element at `at`, as the number the element table holds.
/// The role of each word of a declaration, in order: the name, the parts of the value, and the keyword of its `!important`.
/// A word is a part of the declaration that is no white space and no comment.
#[must_use]
pub(crate) fn declaration_fields(words: &[Word<'_>]) -> Vec<Field> {
    let important = matches!(
        words,
        [.., bang, word]
            if bang.kind == Kind::Token(Token::Delim)
                && bang.spelled == BANG
                && word.kind == Kind::Token(Token::Ident)
                && word.decoded.unwrap_or(word.spelled).eq_ignore_ascii_case(IMPORTANT)
    );
    let value_end = if important {
        words.len().saturating_sub(2)
    } else {
        words.len()
    };
    (0..words.len())
        .map(|place| {
            if place == 0 && words.first().is_some_and(|word| word.kind == Kind::Token(Token::Ident)) {
                Field::Name
            } else if important && place + 1 == words.len() {
                Field::Important
            } else if place >= 2
                && place < value_end
                && words.get(1).is_some_and(|colon| colon.kind == Kind::Token(Token::Colon))
            {
                Field::Value
            } else {
                Field::None
            }
        })
        .collect()
}
