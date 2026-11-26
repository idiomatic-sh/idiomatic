//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the node table holds and back, says its name, and says the role a node of a kind has under its
//! parent.

use ryo::languages;


use super::types::{Field, Kind, Valueness};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

use strum::IntoEnumIterator as _;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

/// Whether the element is a value of the grammar: a string, a number, a literal, an object or an array.
#[must_use]
pub(crate) fn is_value(kind: Kind) -> Valueness {
    match kind {
        Kind::Token(Token::String | Token::Number | Token::True | Token::False | Token::Null)
        | Kind::Node(Node::Object | Node::Array) => Valueness::Value,
        Kind::Node(Node::JsonText | Node::Member)
        | Kind::Token(
            Token::BeginArray
            | Token::BeginObject
            | Token::EndArray
            | Token::EndObject
            | Token::NameSeparator
            | Token::ValueSeparator
            | Token::Ws,
        )
        | Kind::Extra(Extra::Error | Extra::LineComment | Extra::BlockComment) => Valueness::Other,
    }
}

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
