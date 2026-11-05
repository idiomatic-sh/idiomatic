//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the element table holds and back, says its name, and reads the kind of an element of a file's
//! syntax tree: a position is an element of the table; its kind is the language's own.

use ryo::languages::{self, Tree};

use super::constants::{BLOCK_COMMENT_CLOSE, BLOCK_COMMENT_OPEN, LINE_COMMENT_OPEN};
use super::types::{Field, Keyness, Kind, Leaf, Rootness, Text, Valueness};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::JsonNode;
use crate::domain::tokens::types::Token;
use ryo::languages::ValueKind;
use strum::IntoEnumIterator as _;

ryo::languages::kinds!(pub Kind { Node(JsonNode), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

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

/// The text of the comment at `at` without its marks, as a name: after the `//` of a line comment, between the `/*` and the `*/` of a
/// block comment (to the end of the file when it is not closed).
pub(crate) fn comment_text(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let written = written(tree, at).0;
    let inside = match kind(tree, at) {
        Kind::Extra(Extra::BlockComment) => written
            .strip_prefix(BLOCK_COMMENT_OPEN)
            .map(|rest| rest.strip_suffix(BLOCK_COMMENT_CLOSE).unwrap_or(rest)),
        Kind::Extra(Extra::LineComment) => written.strip_prefix(LINE_COMMENT_OPEN),
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(Extra::Error) => None,
    };
    let from = languages::begin_name(symbols);
    languages::push_text(symbols, languages::NameText(inside.unwrap_or_default()));
    languages::end_name(symbols, tree.source, from)
}

/// What a value is, from its text for a number; nothing for an element that is no value.
#[must_use]
pub(crate) fn value_kind(tree: &Tree<'_>, at: languages::Position) -> Option<ValueKind> {
    match kind(tree, at) {
        Kind::Token(Token::String) => Some(ValueKind::String),
        Kind::Token(Token::Number) => Some(if written(tree, at).0.contains(['.', 'e', 'E']) {
            ValueKind::Float
        } else {
            ValueKind::Integer
        }),
        Kind::Token(Token::True | Token::False) => Some(ValueKind::Boolean),
        Kind::Token(Token::Null) => Some(ValueKind::Null),
        Kind::Node(JsonNode::Array) => Some(ValueKind::Sequence),
        Kind::Node(JsonNode::Object) => Some(ValueKind::InlineMapping),
        Kind::Node(JsonNode::JsonText | JsonNode::Member)
        | Kind::Token(
            Token::BeginArray
            | Token::BeginObject
            | Token::EndArray
            | Token::EndObject
            | Token::NameSeparator
            | Token::ValueSeparator
            | Token::Ws,
        )
        | Kind::Extra(Extra::Error | Extra::LineComment | Extra::BlockComment) => None,
    }
}

/// Whether the element is a value of the grammar: a string, a number, a literal, an object or an array.
#[must_use]
pub(crate) fn is_value(kind: Kind) -> Valueness {
    match kind {
        Kind::Token(Token::String | Token::Number | Token::True | Token::False | Token::Null)
        | Kind::Node(JsonNode::Object | JsonNode::Array) => Valueness::Value,
        Kind::Node(JsonNode::JsonText | JsonNode::Member)
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

/// Whether the string at `at` is the name of a member: the first string of its member.
#[must_use]
pub(crate) fn keyness(tree: &Tree<'_>, at: languages::Position) -> Keyness {
    let member = parent(tree, at);
    if kind(tree, member) == Kind::Node(JsonNode::Member)
        && languages::child_of_kind(tree.elements, member, kind_bits(Kind::Token(Token::String)))
            == Some(at)
    {
        Keyness::Key
    } else {
        Keyness::Other
    }
}

/// The value of the member at `member`: its last value, after the colon.
#[must_use]
pub(crate) fn value_of(
    tree: &Tree<'_>,
    member: languages::Position,
) -> Option<languages::Position> {
    languages::children(tree.elements, member)
        .filter(|child| is_value(kind(tree, *child)) == Valueness::Value)
        .last()
}

/// The element a container, an object or an array, stands for as the value that holds it: the file for a container that is the whole
/// document, the name of its member, or itself when it is an element of an array.
#[must_use]
pub(crate) fn holder(tree: &Tree<'_>, container: languages::Position) -> languages::Position {
    let above = parent(tree, container);
    match kind(tree, above) {
        Kind::Node(JsonNode::JsonText) => above,
        Kind::Node(JsonNode::Member) => {
            languages::child_of_kind(tree.elements, above, kind_bits(Kind::Token(Token::String)))
                .unwrap_or(container)
        }
        Kind::Node(JsonNode::Object | JsonNode::Array) | Kind::Token(_) | Kind::Extra(_) => container,
    }
}

/// The element that stands for the object at `object` in the rows that name a mapping: the file for the object that is the whole
/// document, the object otherwise.
#[must_use]
pub(crate) fn mapping_id(tree: &Tree<'_>, object: languages::Position) -> languages::Position {
    let above = parent(tree, object);
    if kind(tree, above) == Kind::Node(JsonNode::JsonText) {
        above
    } else {
        object
    }
}

/// What the file's library decoded of the token at `at`, as a name; nothing for a token it did not decode.
pub(crate) fn decoded(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> Option<languages::Symbol> {
    let text = languages::decoded_at(tree.elements, at)?;
    let name = languages::begin_name(symbols);
    languages::push_text(symbols, text);
    Some(languages::end_name(symbols, tree.source, name))
}

/// What the string value at `at` says, as the library decoded it; no name for a value that is no string.
pub(crate) fn said(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    if kind(tree, at) == Kind::Token(Token::String) {
        decoded(tree, symbols, at).unwrap_or(languages::NO_SYMBOL)
    } else {
        languages::NO_SYMBOL
    }
}

/// The value at `at` as a yes or no: raised for `true`; lowered for `false` and for a value of every other kind. The library scans the
/// two literals as tokens of their own, so the token's kind is the value.
#[must_use]
pub(crate) fn boolean(tree: &Tree<'_>, at: languages::Position) -> languages::Flag {
    if kind(tree, at) == Kind::Token(Token::True) {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// The value at `at` as a whole number when it is an integer: the library hands a number over as the text it scanned, which the
/// grammar it reads with (no hexadecimal, no unary plus) holds to an optional minus and decimal digits. Zero for a value of every
/// other kind, and for an integer past the range of the engine's numbers, whose `text` keeps it.
#[must_use]
pub(crate) fn integer(tree: &Tree<'_>, at: languages::Position) -> languages::Integer {
    languages::Integer(if value_kind(tree, at) == Some(ValueKind::Integer) {
        written(tree, at).0.parse::<i64>().unwrap_or_default()
    } else {
        0
    })
}

/// How a mapping is written: the whole document, the value of a member, or an element of an array.
#[must_use]
pub(crate) fn mapping_kind(tree: &Tree<'_>, object: languages::Position) -> languages::MappingKind {
    match kind(tree, parent(tree, object)) {
        Kind::Node(JsonNode::JsonText) => languages::MappingKind::Standard,
        Kind::Node(JsonNode::Array) => languages::MappingKind::SequenceMember,
        Kind::Node(JsonNode::Object | JsonNode::Member) | Kind::Token(_) | Kind::Extra(_) => {
            languages::MappingKind::Inline
        }
    }
}

/// What the library decoded of the token at `at` as a name, or the text the token spans when it decoded nothing.
pub(crate) fn decoded_name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    decoded(tree, symbols, at).unwrap_or_else(|| spanned(tree, symbols, at))
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

/// The role of the element at `at` under its parent, by the part of RFC 8259's grammar it is.
#[must_use]
pub(crate) fn field_of(tree: &Tree<'_>, at: languages::Position) -> Field {
    if is_root(tree, at) == Rootness::Root {
        return Field::None;
    }
    let holder = parent(tree, at);
    match kind(tree, holder) {
        Kind::Node(JsonNode::Member) => {
            if keyness(tree, at) == Keyness::Key {
                Field::Name
            } else if value_of(tree, holder) == Some(at) {
                Field::Value
            } else {
                Field::None
            }
        }
        Kind::Node(JsonNode::Array | JsonNode::JsonText)
            if is_value(kind(tree, at)) == Valueness::Value =>
        {
            Field::Value
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => Field::None,
    }
}

/// The role of the element at `at`, as the number the element table holds.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: languages::Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}
