//! The `walked::syntax` service: free functions over a walked file's node table. It moves about a file's syntax tree as the services
//! over today's reading do: a position is an element of the table, its kind is the language's own.

use crate::domain::file_view::types::{Syntax as Tree, Words};
use crate::domain::syntax::service::{kind_bits, kind_of};
use crate::domain::syntax::types::{Kind, Leaf, Rootness, Spelled, Text};
use crate::domain::tokens::types::Token;
use ryo::languages::{self, NameText, Position, Symbol, node_table};

/// The kind of the element at `at`.
#[must_use]
pub(crate) fn kind(tree: &Tree<'_>, at: Position) -> Kind {
    kind_of(node_table::kind_of(tree.table, at))
}

/// Whether the element at `at` is a token, a leaf, and not a node: the tokens are numbered after the nodes.
#[must_use]
pub(crate) fn is_token(tree: &Tree<'_>, at: Position) -> Leaf {
    if usize::from(at) >= usize::try_from(node_table::node_count(tree.table).0).unwrap_or(usize::MAX) {
        Leaf::Token
    } else {
        Leaf::Node
    }
}

/// The parent of the element at `at`; the root is its own.
#[must_use]
pub(crate) fn parent(tree: &Tree<'_>, at: Position) -> Position {
    node_table::parent_of(tree.table, at)
}

/// Whether `at` is the root of the tree.
#[must_use]
pub(crate) fn is_root(tree: &Tree<'_>, at: Position) -> Rootness {
    if parent(tree, at) == at {
        Rootness::Root
    } else {
        Rootness::Inner
    }
}

/// The place of the element at `at` among its parent's children.
#[must_use]
pub(crate) fn ordinal(tree: &Tree<'_>, at: Position) -> languages::Integer {
    languages::Integer(i64::from(node_table::ordinal_of(tree.table, at).0))
}

/// The elements directly under `at`, in order.
pub(crate) fn children<'f>(tree: &Tree<'f>, at: Position) -> impl Iterator<Item = Position> + use<'f> {
    let table: &'f languages::NodeTable = tree.table;
    node_table::children_of(table, at).iter().copied()
}

/// The nodes under the element at `at`, in order: its children that are no token.
pub(crate) fn nodes<'f>(tree: &Tree<'f>, at: Position) -> impl Iterator<Item = Position> + use<'f> {
    let tree = *tree;
    children(&tree, at).filter(move |each| is_token(&tree, *each) == Leaf::Node)
}

/// The elements above the element at `at`, nearest first, up to and including the root.
pub(crate) fn ancestors<'f>(tree: &Tree<'f>, at: Position) -> impl Iterator<Item = Position> + use<'f> {
    let tree = *tree;
    let mut next = (is_root(&tree, at) == Rootness::Inner).then(|| parent(&tree, at));
    std::iter::from_fn(move || {
        let current = next?;
        next = (is_root(&tree, current) == Rootness::Inner).then(|| parent(&tree, current));
        Some(current)
    })
}

/// The tokens under the element at `at`, in order: its children that are tokens, comments among them.
pub(crate) fn tokens<'f>(tree: &Tree<'f>, at: Position) -> impl Iterator<Item = Spelled> + use<'f> {
    let tree = *tree;
    children(&tree, at).filter_map(move |each| match kind(&tree, each) {
        Kind::Token(token) => Some(Spelled { at: each, token }),
        Kind::Node(_) | Kind::Extra(_) => None,
    })
}

/// The first token under the element at `at` that is `wanted`.
#[must_use]
pub(crate) fn token(tree: &Tree<'_>, at: Position, wanted: Token) -> Option<Position> {
    tokens(tree, at).find(|each| each.token == wanted).map(|each| each.at)
}

/// The first child of the element at `at` whose kind is `wanted`.
#[must_use]
pub(crate) fn child(tree: &Tree<'_>, at: Position, wanted: Kind) -> Option<Position> {
    let wanted = kind_bits(wanted);
    children(tree, at).find(|each| node_table::kind_of(tree.table, *each) == wanted)
}

/// The text the element at `at` spans, as it is written in the file.
#[must_use]
pub(crate) fn written<'f>(tree: &Tree<'f>, at: Position) -> Text<'f> {
    let span = node_table::span_of(tree.table, at);
    let (start, end) = (
        usize::try_from(span.start.0).unwrap_or(usize::MAX),
        usize::try_from(span.end.0).unwrap_or(usize::MAX),
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

/// The start of the element at `at`.
#[must_use]
pub(crate) fn start_at(tree: &Tree<'_>, at: Position) -> languages::Offset {
    node_table::span_of(tree.table, at).start
}

/// The end of the element at `at`.
#[must_use]
pub(crate) fn end_at(tree: &Tree<'_>, at: Position) -> languages::Offset {
    node_table::span_of(tree.table, at).end
}

/// What the library decoded of the token at `at`.
#[must_use]
pub(crate) fn decoded_at<'f>(tree: &Tree<'f>, at: Position) -> Option<&'f str> {
    node_table::decoded_of(tree.table, at).map(|text| text.0)
}

/// A text as a name of the file.
pub(crate) fn word(words: &Words<'_>, text: &str) -> Symbol {
    Symbol(u32::try_from(languages::facts::intern(&mut words.0.borrow_mut(), NameText(text)).0).unwrap_or(u32::MAX))
}

/// The text of the token at `at`, as a name; no text for a node.
pub(crate) fn token_text(tree: &Tree<'_>, words: &Words<'_>, at: Position) -> Symbol {
    if is_token(tree, at) == Leaf::Token {
        word(words, written(tree, at).0)
    } else {
        languages::NO_SYMBOL
    }
}

/// The text the element at `at` spans, as a name.
pub(crate) fn spanned(tree: &Tree<'_>, words: &Words<'_>, at: Position) -> Symbol {
    word(words, written(tree, at).0)
}

/// The text of the first token among the children of the element at `of`: the name a node that holds one holds.
pub(crate) fn first_token_text(tree: &Tree<'_>, words: &Words<'_>, of: Position) -> Option<Symbol> {
    children(tree, of)
        .find(|each| is_token(tree, *each) == Leaf::Token)
        .map(|each| word(words, written(tree, each).0))
}

/// The element `at` as a row's node cell.
#[must_use]
pub(crate) fn node_cell(tree: &Tree<'_>, at: Position) -> languages::NodeId {
    languages::node_id(tree.file, at)
}

/// Where an element is, as a row's span.
#[must_use]
pub(crate) fn span_of(tree: &Tree<'_>, at: Position) -> languages::Span {
    let span = node_table::span_of(tree.table, at);
    languages::Span { start: span.start, end: span.end }
}

/// Whether the element `first` comes before its sibling `second`: the tokens are numbered after the nodes, so two siblings are told
/// by their places among the children of their parent.
#[must_use]
pub(crate) fn before(tree: &Tree<'_>, first: Position, second: Position) -> bool {
    node_table::ordinal_of(tree.table, first).0 < node_table::ordinal_of(tree.table, second).0
}
