//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the element table holds and back, says its name, and moves
//! about a file's syntax tree: a position is an element of the table; its kind is the language's own.

use ryo::languages::{self};

use super::types::{Field, Kind};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use ryo::languages::ValueKind;
use strum::IntoEnumIterator as _;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

/// What a value of `kind` is; nothing for an element that is no value.
#[must_use]
pub(crate) fn value_kind(kind: Kind) -> Option<ValueKind> {
    match kind {
        Kind::Token(
            Token::BasicString
            | Token::LiteralString
            | Token::MlBasicString
            | Token::MlLiteralString,
        ) => Some(ValueKind::String),
        Kind::Token(Token::Integer) => Some(ValueKind::Integer),
        Kind::Token(Token::Float) => Some(ValueKind::Float),
        Kind::Token(Token::Boolean) => Some(ValueKind::Boolean),
        Kind::Token(Token::DateTime) => Some(ValueKind::Datetime),
        Kind::Node(Node::Array) => Some(ValueKind::Sequence),
        Kind::Node(Node::InlineTable) => Some(ValueKind::InlineMapping),
        Kind::Node(Node::Toml | Node::StdTable | Node::ArrayTable | Node::Keyval | Node::Key)
        | Kind::Token(
            Token::UnquotedKey
            | Token::Comment
            | Token::Ws
            | Token::Newline
            | Token::KeyvalSep
            | Token::DotSep
            | Token::ArraySep
            | Token::InlineTableSep
            | Token::StdTableOpen
            | Token::StdTableClose
            | Token::ArrayOpen
            | Token::ArrayClose
            | Token::ArrayTableOpen
            | Token::ArrayTableClose
            | Token::InlineTableOpen
            | Token::InlineTableClose,
        )
        | Kind::Extra(Extra::Error) => None,
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

/// The role of an element of `element` kind under a parent of `parent` kind, by the element of `toml.abnf` it is in its parent's rule.
#[must_use]
pub(crate) fn field_between(parent: Kind, element: Kind) -> Field {
    match parent {
        Kind::Node(Node::Keyval) if element == Kind::Node(Node::Key) => Field::Key,
        Kind::Node(Node::Keyval | Node::Array) if value_kind(element).is_some() => Field::Val,
        Kind::Node(Node::StdTable | Node::ArrayTable) if element == Kind::Node(Node::Key) => {
            Field::Key
        }
        Kind::Node(Node::Key)
            if matches!(
                element,
                Kind::Token(Token::UnquotedKey | Token::BasicString | Token::LiteralString)
            ) =>
        {
            Field::SimpleKey
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => Field::None,
    }
}
