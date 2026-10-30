//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no ports. It maps a kind to the number the
//! element table holds and back, says its name, and moves about a file's syntax tree: a position is an element of the table; its kind
//! is the language's own.

use ryo::languages::{self, Tree};

use super::types::{Kind, Leaf, Rootness, Spelled, Text};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Token(Token::Illegal));

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

/// The nodes under the element at `at`, in order: its children that are no token.
pub(crate) fn nodes<'a>(
    tree: &'a Tree<'_>,
    at: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    languages::children(tree.elements, at).filter(move |each| is_token(tree, *each) == Leaf::Node)
}

/// The elements above the element at `at`, nearest first, up to and including the root.
pub(crate) fn ancestors<'a>(
    tree: &'a Tree<'_>,
    at: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    let mut next = (is_root(tree, at) == Rootness::Inner).then(|| parent(tree, at));
    std::iter::from_fn(move || {
        let current = next?;
        next = (is_root(tree, current) == Rootness::Inner).then(|| parent(tree, current));
        Some(current)
    })
}

/// The tokens under the element at `at`, in order: its children that are tokens, comments among them.
pub(crate) fn tokens<'a>(
    tree: &'a Tree<'_>,
    at: languages::Position,
) -> impl Iterator<Item = Spelled> + 'a {
    languages::children(tree.elements, at).filter_map(move |each| match kind(tree, each) {
        Kind::Token(token) => Some(Spelled { at: each, token }),
        Kind::Node(_) | Kind::Extra(_) => None,
    })
}

/// The first token under the element at `at` that is `wanted`.
#[must_use]
pub(crate) fn token(
    tree: &Tree<'_>,
    at: languages::Position,
    wanted: Token,
) -> Option<languages::Position> {
    tokens(tree, at)
        .find(|each| each.token == wanted)
        .map(|each| each.at)
}

/// The first child of the element at `at` whose kind is `wanted`.
#[must_use]
pub(crate) fn child(
    tree: &Tree<'_>,
    at: languages::Position,
    wanted: Kind,
) -> Option<languages::Position> {
    languages::child_of_kind(tree.elements, at, kind_bits(wanted))
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
            .and_then(|bytes| std::str::from_utf8(bytes).ok()) // ignore: idiomatic.arch/no-ok-on-result - the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
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
