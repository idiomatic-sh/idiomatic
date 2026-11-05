//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no ports. It maps a kind to the number
//! the element table holds and back, says its name, and moves about a file's syntax tree: a position is an element of the table; its
//! kind is the language's own.

use ryo::languages::{self, Tree};

use super::constants::{BANG, IMPORTANT};
use super::types::{Field, Kind, Leaf, Rootness};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::CssNode;
use crate::domain::tokens::types::Token;
use strum::IntoEnumIterator as _;

ryo::languages::kinds!(pub Kind { Node(CssNode), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

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

/// What the token at `at` says as the library decoded it (a string without its quotes and with its escapes resolved, a comment
/// without its `/*` and `*/`, a name), as a name; nothing for a token the library decodes nothing of.
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

/// The bytes of the element at `at`, as the file spells them.
fn spelled<'a>(tree: &Tree<'a>, at: languages::Position) -> languages::NameText<'a> {
    let from = usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX);
    let to = usize::try_from(languages::end_at(tree.elements, at).0).unwrap_or(usize::MAX);
    languages::NameText(
        tree.source
            .0
            .get(from..to)
            .and_then(|bytes| std::str::from_utf8(bytes).ok()) // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
            .unwrap_or_default(),
    )
}

/// The role of the element at `at` under its parent. Under a declaration the children that are not white space or comments are
/// its name, then its colon, then its value, and last, when the value ends with a `!` delim-token and an ident-token `important`
/// in any case (CSS Syntax 5.4.6, "consume a declaration"), its importance: the name is in the role `name`, each component value of
/// the value in `value`, and the `important` in `important`. Every other edge has none.
#[must_use]
pub(crate) fn field_of(tree: &Tree<'_>, at: languages::Position) -> Field {
    let declaration = parent(tree, at);
    if declaration == at || kind(tree, declaration) != Kind::Node(CssNode::Declaration) {
        return Field::None;
    }
    let words: Vec<languages::Position> = languages::children(tree.elements, declaration)
        .filter(|each| {
            !matches!(
                kind(tree, *each),
                Kind::Token(Token::Whitespace) | Kind::Extra(Extra::Comment)
            )
        })
        .collect();
    let Some(place) = words.iter().position(|each| *each == at) else {
        return Field::None;
    };
    let important = matches!(
        words.as_slice(),
        [.., bang, word]
            if kind(tree, *bang) == Kind::Token(Token::Delim)
                && spelled(tree, *bang).0 == BANG
                && kind(tree, *word) == Kind::Token(Token::Ident)
                && languages::decoded_at(tree.elements, *word)
                    .unwrap_or_else(|| spelled(tree, *word))
                    .0
                    .eq_ignore_ascii_case(IMPORTANT)
    );
    let value_end = if important {
        words.len().saturating_sub(2)
    } else {
        words.len()
    };
    if place == 0 && kind(tree, at) == Kind::Token(Token::Ident) {
        Field::Name
    } else if important && place + 1 == words.len() {
        Field::Important
    } else if place >= 2
        && place < value_end
        && words
            .get(1)
            .is_some_and(|colon| kind(tree, *colon) == Kind::Token(Token::Colon))
    {
        Field::Value
    } else {
        Field::None
    }
}

/// The role of the element at `at`, as the number the element table holds.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: languages::Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}
