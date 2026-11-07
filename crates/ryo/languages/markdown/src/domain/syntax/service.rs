//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the element table holds and back, says its name, and moves
//! about a file's syntax tree: a position is an element of the table; its kind is the language's own. It says what an HTML
//! comment holds and what code the library decoded.

use ryo::languages::{self, Tree};

use super::constants::{COMMENT_CLOSE, COMMENT_OPEN};
use super::types::{Commentness, Field, Kind, Leaf, Level, Rootness, Text};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use strum::IntoEnumIterator as _;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

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
            // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
            .map(|bytes| std::str::from_utf8(bytes).unwrap_or_default())
            .unwrap_or_default(),
    )
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

/// Whether the raw HTML at `at` (a block, or HTML in a line) is a comment: whether it opens with `<!--`, after the indentation.
#[must_use]
pub(crate) fn commentness(tree: &Tree<'_>, at: languages::Position) -> Commentness {
    if written(tree, at).0.trim_start().starts_with(COMMENT_OPEN) {
        Commentness::Comment
    } else {
        Commentness::Markup
    }
}

/// The text of the HTML comment at `at` between its `<!--` and its first `-->` (to the end, when it is never closed), as a name.
pub(crate) fn comment_text(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let text = written(tree, at).0.trim_start();
    let inner = text.get(COMMENT_OPEN.len()..).unwrap_or_default();
    let inner = inner.split(COMMENT_CLOSE).next().unwrap_or_default();
    let from = languages::begin_name(symbols);
    languages::push_text(symbols, languages::NameText(inner));
    languages::end_name(symbols, tree.source, from)
}

/// The code at `at`, as the library decoded it, as a name: for code in a line of text, what the library read between its
/// backticks; for a block of code, the lines of code the library read, in order, without their fences, indentation or markers.
pub(crate) fn code_value(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let from = languages::begin_name(symbols);
    if kind(tree, at) == Kind::Node(Node::InlineCode) {
        if let Some(text) = languages::decoded_at(tree.elements, at) {
            languages::push_text(symbols, text);
        }
    } else {
        for child in languages::children(tree.elements, at) {
            if kind(tree, child) == Kind::Token(Token::CodeFlowValue)
                && let Some(text) = languages::decoded_at(tree.elements, child)
            {
                languages::push_text(symbols, text);
            }
        }
    }
    languages::end_name(symbols, tree.source, from)
}

/// How deep the heading at `at` is, which the adapter wrote as the text of its first marker: `#` marks and an underline are both marked
/// with their level. None for an element that is no heading.
#[must_use]
pub(crate) fn level_of(tree: &Tree<'_>, at: languages::Position) -> Option<Level> {
    if kind(tree, at) != Kind::Node(Node::Heading) {
        return None;
    }
    named_by(tree, at)
        .and_then(|marked| {
            // a heading whose level the library writes as no number has no level, which the caller reads as no scope
            let Ok(level) = marked.0.parse() else {
                return None;
            };
            Some(level)
        })
        .map(Level)
}

/// What the construct at `at` names, which the adapter wrote on its first marker: the target of a link or an image, resolved to a
/// path of the run, and the label of a footnote's definition. None for a construct that names nothing.
#[must_use]
pub(crate) fn named_by<'a>(
    tree: &Tree<'a>,
    at: languages::Position,
) -> Option<languages::NameText<'a>> {
    let marker = languages::children(tree.elements, at)
        .find(|child| kind(tree, *child) == Kind::Extra(Extra::Markup))?;
    languages::decoded_at(tree.elements, marker)
}

/// The label a definition of a link reference gives, which the adapter wrote on it.
#[must_use]
pub(crate) fn labelled<'a>(
    tree: &Tree<'a>,
    at: languages::Position,
) -> Option<languages::NameText<'a>> {
    languages::decoded_at(tree.elements, at)
}

/// `text` as a name of the file.
pub(crate) fn name_of(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    text: languages::NameText<'_>,
) -> languages::Symbol {
    let from = languages::begin_name(symbols);
    languages::push_text(symbols, text);
    languages::end_name(symbols, tree.source, from)
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

/// The role of the element at `at` under its parent, by the property of mdast's node that holds it.
#[must_use]
pub(crate) fn field_of(tree: &Tree<'_>, at: languages::Position) -> Field {
    if is_root(tree, at) == Rootness::Root {
        return Field::None;
    }
    match (kind(tree, parent(tree, at)), kind(tree, at)) {
        (Kind::Node(Node::Code), Kind::Token(Token::CodeFlowValue))
        | (Kind::Node(Node::Html), Kind::Token(Token::HtmlFlowData)) => Field::Value,
        (Kind::Node(Node::ListItem), Kind::Extra(Extra::TaskChecked | Extra::TaskUnchecked)) => {
            Field::Checked
        }
        (
            _,
            Kind::Node(_)
            | Kind::Extra(
                Extra::Yaml
                | Extra::Toml
                | Extra::Math
                | Extra::InlineMath
                | Extra::DefinitionList
                | Extra::DefinitionListTitle
                | Extra::DefinitionListDefinition
                | Extra::Superscript
                | Extra::Subscript,
            ),
        ) => Field::Children,
        (
            _,
            Kind::Token(_)
            | Kind::Extra(
                Extra::Markup
                | Extra::TaskChecked
                | Extra::TaskUnchecked
                | Extra::Error
                | Extra::Section,
            ),
        ) => Field::None,
    }
}

/// The role of the element at `at`, as the number the element table holds.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: languages::Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}
