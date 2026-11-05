//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the element table holds and back, says its name, and moves
//! about a file's syntax tree: a position is an element of the table; its kind is the language's own.

use ryo::languages::{self, Tree};

use super::types::{Field, Kind, Leaf, Rootness, Text};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::TomlNode;
use crate::domain::tokens::types::Token;
use ryo::languages::ValueKind;
use strum::IntoEnumIterator as _;

ryo::languages::kinds!(pub Kind { Node(TomlNode), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

/// The kind of the element at `at`.
#[must_use]
pub(crate) fn kind(tree: &Tree<'_>, at: languages::Position) -> Kind {
    kind_of(languages::kind_at(tree.elements, at))
}

/// Whether the element at `at` is a token, a leaf, and not a node.
#[must_use]
pub(crate) fn is_token(tree: &Tree<'_>, at: languages::Position) -> Leaf {
    if languages::flags_at(tree.elements, at).0 & languages::TOKEN.0 == 0 {
        Leaf::Node
    } else {
        Leaf::Token
    }
}

/// The parent of the element at `at`; the root is its own.
#[must_use]
pub(crate) fn parent(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    languages::parent_at(tree.elements, at)
}

/// Whether `at` is the root of the tree.
#[must_use]
pub(crate) fn is_root(tree: &Tree<'_>, at: languages::Position) -> Rootness {
    if parent(tree, at) == at {
        Rootness::Root
    } else {
        Rootness::Inner
    }
}

/// The place of the element at `at` among its parent's children.
#[must_use]
pub(crate) fn ordinal(tree: &Tree<'_>, at: languages::Position) -> languages::Integer {
    languages::Integer(i64::from(languages::ordinal_at(tree.elements, at).0))
}

/// The name of the kind of the element at `at`, as the `kind` column of the tree holds it.
pub(crate) fn kind_symbol(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let element = kind(tree, at);
    languages::vocabulary_symbol(symbols, tree.source, kind_bits(element), kind_name(element))
}

/// The text of the token at `at`; no text for a node.
#[must_use]
pub(crate) fn token_text(tree: &Tree<'_>, at: languages::Position) -> languages::Symbol {
    if is_token(tree, at) == Leaf::Token {
        languages::text_at(tree.elements, at)
    } else {
        languages::NO_SYMBOL
    }
}

/// The text the element at `at` spans, as it is written in the file.
#[must_use]
pub(crate) fn written<'a>(tree: &Tree<'a>, at: languages::Position) -> Text<'a> {
    let (start, end) = (
        usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX),
        usize::try_from(languages::end_at(tree.elements, at).0).unwrap_or(usize::MAX),
    );
    Text(
        tree.source
            .0
            .get(start..end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok()) // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
            .unwrap_or_default(),
    )
}

/// The text the element at `at` spans, as a name: a token's own text, or the text a node spans.
pub(crate) fn spanned(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    if is_token(tree, at) == Leaf::Token {
        return languages::text_at(tree.elements, at);
    }
    let from = languages::begin_name(symbols);
    languages::push_text(symbols, languages::NameText(written(tree, at).0));
    languages::end_name(symbols, tree.source, from)
}

/// The place of the `number`-th thing the parser reported about the file, as a node id of the file: a report is known by its number.
#[must_use]
pub(crate) fn diagnostic_id(tree: &Tree<'_>, number: languages::ChildOrdinal) -> languages::NodeId {
    languages::diagnostic_id(tree.file, number)
}

/// What the parser said at one place, as a name.
pub(crate) fn diagnostic_message(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    diagnostic: &languages::Diagnostic<'_>,
) -> languages::Symbol {
    languages::intern_derived(
        symbols,
        tree.source,
        languages::NameText(std::str::from_utf8(diagnostic.message.0).unwrap_or_default()),
    )
}

/// Where the parser said it.
#[must_use]
pub(crate) fn diagnostic_span(diagnostic: &languages::Diagnostic<'_>) -> languages::Span {
    languages::Span {
        start: diagnostic.start,
        end: diagnostic.end,
    }
}

/// The text of the comment at `at` after its `#`, as a name.
pub(crate) fn comment_text(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let from = languages::begin_name(symbols);
    languages::push_text(
        symbols,
        languages::NameText(written(tree, at).0.get(1..).unwrap_or_default()),
    );
    languages::end_name(symbols, tree.source, from)
}

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
        Kind::Node(TomlNode::Array) => Some(ValueKind::Sequence),
        Kind::Node(TomlNode::InlineTable) => Some(ValueKind::InlineMapping),
        Kind::Node(TomlNode::Toml | TomlNode::StdTable | TomlNode::ArrayTable | TomlNode::Keyval | TomlNode::Key)
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

/// The role of the element at `at` under its parent, by the element of `toml.abnf` it is in its parent's rule.
#[must_use]
pub(crate) fn field_of(tree: &Tree<'_>, at: languages::Position) -> Field {
    if is_root(tree, at) == Rootness::Root {
        return Field::None;
    }
    let element = kind(tree, at);
    match kind(tree, parent(tree, at)) {
        Kind::Node(TomlNode::Keyval) if element == Kind::Node(TomlNode::Key) => Field::Key,
        Kind::Node(TomlNode::Keyval | TomlNode::Array) if value_kind(element).is_some() => Field::Val,
        Kind::Node(TomlNode::StdTable | TomlNode::ArrayTable) if element == Kind::Node(TomlNode::Key) => {
            Field::Key
        }
        Kind::Node(TomlNode::Key)
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

/// The role of the element at `at`, as the number the element table holds.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: languages::Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}
