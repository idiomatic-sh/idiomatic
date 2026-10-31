//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no ports. It moves about a file's
//! syntax tree: a position is an element of the table; its kind is a node, a token or an extra, by the standards' names.

use ryo::languages::{self, Position, Tree};

use super::types::{Interpolation, Kind, Leaf, Literal, Rootness, Standing, Text};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::service::category as node_category;
use crate::domain::nodes::types::{Category, Node};
use crate::domain::tokens::types::Token;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Token(Token::Errortoken));

/// The standard's category of a kind; `Other` for a token or an extra.
#[must_use]
pub(crate) const fn category_of(kind: Kind) -> Category {
    match kind {
        Kind::Node(each) => node_category(each),
        Kind::Token(_) | Kind::Extra(_) => Category::Other,
    }
}

/// The kind of the element at `at`.
#[must_use]
pub(crate) fn kind(tree: &Tree<'_>, at: Position) -> Kind {
    kind_of(languages::kind_at(tree.elements, at))
}

/// The standard's category of the element at `at`.
#[must_use]
pub(crate) fn category(tree: &Tree<'_>, at: Position) -> Category {
    category_of(kind(tree, at))
}

/// Whether the element at `at` is a token, a leaf, and not a node.
#[must_use]
pub(crate) fn is_token(tree: &Tree<'_>, at: Position) -> Leaf {
    if languages::flags_at(tree.elements, at).0 & languages::TOKEN.0 == 0 {
        Leaf::Node
    } else {
        Leaf::Token
    }
}

/// The parent of the element at `at`; the root is its own.
#[must_use]
pub(crate) fn parent(tree: &Tree<'_>, at: Position) -> Position {
    languages::parent_at(tree.elements, at)
}

/// The last element below the element at `at`, in pre-order; `at` itself when it has none.
#[must_use]
pub(crate) fn last(tree: &Tree<'_>, at: Position) -> Position {
    languages::last_at(tree.elements, at)
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
    languages::Integer(i64::from(languages::ordinal_at(tree.elements, at).0))
}

/// The nodes under the element at `at`, in order: its children that are no token.
pub(crate) fn nodes<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    languages::children(tree.elements, at).filter(move |each| is_token(tree, *each) == Leaf::Node)
}

/// The nodes under the element at `at` that are of the category `wanted`, in order.
pub(crate) fn nodes_of<'a>(
    tree: &'a Tree<'_>,
    at: Position,
    wanted: Category,
) -> impl Iterator<Item = Position> + 'a {
    nodes(tree, at).filter(move |each| category(tree, *each) == wanted)
}

/// The first child of the element at `at` whose kind is `wanted`.
#[must_use]
pub(crate) fn child(tree: &Tree<'_>, at: Position, wanted: Kind) -> Option<Position> {
    languages::child_of_kind(tree.elements, at, kind_bits(wanted))
}

/// The name of the kind of the element at `at`, as the `kind` column of the tree holds it.
pub(crate) fn kind_symbol(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> languages::Symbol {
    let element = kind(tree, at);
    languages::vocabulary_symbol(symbols, tree.source, kind_bits(element), kind_name(element))
}

/// The text of the token at `at`; no text for a node.
#[must_use]
pub(crate) fn token_text(tree: &Tree<'_>, at: Position) -> languages::Symbol {
    if is_token(tree, at) == Leaf::Token {
        languages::text_at(tree.elements, at)
    } else {
        languages::NO_SYMBOL
    }
}

/// The text the element at `at` spans, as it is written in the file.
#[must_use]
pub(crate) fn written<'a>(tree: &Tree<'a>, at: Position) -> Text<'a> {
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
    at: Position,
) -> languages::Symbol {
    if is_token(tree, at) == Leaf::Token {
        return languages::text_at(tree.elements, at);
    }
    let from = languages::begin_name(symbols);
    languages::push_text(symbols, languages::NameText(written(tree, at).0));
    languages::end_name(symbols, tree.source, from)
}

/// The first `NAME` token under the element at `at`, at any depth: the name an identifier or a name expression writes.
#[must_use]
pub(crate) fn name_token(tree: &Tree<'_>, at: Position) -> Option<Position> {
    (at.0..=last(tree, at).0)
        .map(Position)
        .find(|each| kind(tree, *each) == Kind::Token(Token::Name))
}

/// The name the identifier or name expression at `at` writes, as a name.
#[must_use]
pub(crate) fn name_of(tree: &Tree<'_>, at: Position) -> Option<languages::Symbol> {
    let token = name_token(tree, at)?;
    let owner = parent(tree, token);
    if languages::property_at(
        tree.elements,
        owner,
        super::constants::NORMALIZED_IDENTIFIER,
    )
    .is_some()
    {
        Some(languages::text_at(tree.elements, owner))
    } else {
        Some(languages::text_at(tree.elements, token))
    }
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
    at: Position,
) -> languages::Symbol {
    let from = languages::begin_name(symbols);
    languages::push_text(
        symbols,
        languages::NameText(written(tree, at).0.get(1..).unwrap_or_default()),
    );
    languages::end_name(symbols, tree.source, from)
}

/// The positions below the element at `at`, in order.
pub(crate) fn below(tree: &Tree<'_>, at: Position) -> impl Iterator<Item = Position> + use<> {
    (at.0 + 1..=last(tree, at).0).map(Position)
}

/// Whether the string, f-string or t-string at `at` interpolates a value anywhere inside it.
#[must_use]
pub(crate) fn interpolation(tree: &Tree<'_>, at: Position) -> Interpolation {
    let interpolates = below(tree, at).any(|each| {
        matches!(
            kind(tree, each),
            Kind::Node(Node::FormattedValue | Node::Interpolation)
        )
    });
    if interpolates {
        Interpolation::Interpolates
    } else {
        Interpolation::Static
    }
}

/// What the `Constant` at `at` is: a string is made of string parts, a number of a `NUMBER` token, anything else is neither.
#[must_use]
pub(crate) fn literal(tree: &Tree<'_>, at: Position) -> Literal {
    let made_of = |part: Kind| child(tree, at, part).is_some();
    if kind(tree, at) != Kind::Node(Node::Constant) {
        Literal::Other
    } else if made_of(Kind::Extra(Extra::StringPart)) {
        Literal::String
    } else if made_of(Kind::Token(Token::Number)) {
        Literal::Number
    } else {
        Literal::Other
    }
}

/// Whether the f-string at `at` is the format spec of an interpolation, `f"{x:>10}"`, part of a string, or a string of its own: a
/// spec is written in no quotes of its own, so it holds no part of a string; an f-string interpolated, `f"{f'{x}'}"`, does.
#[must_use]
pub(crate) fn standing(tree: &Tree<'_>, at: Position) -> Standing {
    let quoted = [Extra::FStringPart, Extra::TStringPart]
        .into_iter()
        .any(|part| child(tree, at, Kind::Extra(part)).is_some());
    let interpolated = matches!(
        kind(tree, parent(tree, at)),
        Kind::Node(Node::FormattedValue | Node::Interpolation)
    );
    if interpolated && !quoted {
        Standing::FormatSpec
    } else {
        Standing::Own
    }
}

/// A semantic identifier's canonical spelling; raw syntax tokens retain the original spelling.
#[must_use]
pub(crate) fn identifier_text<'a>(tree: &'a Tree<'a>, at: Position) -> Text<'a> {
    languages::property_at(tree.elements, at, super::constants::NORMALIZED_IDENTIFIER)
        .map_or_else(|| written(tree, at), |name| Text(name.0))
}

/// All lexical identifier fields and name expressions in a complete source tree.
pub(crate) fn identifiers<'a>(tree: &'a Tree<'a>) -> Option<impl Iterator<Item = Position> + 'a> {
    if languages::diagnostics(tree.elements).next().is_some() {
        return None;
    }
    Some(
        (0..languages::end_of(tree.elements).0)
            .map(Position)
            .filter(|at| {
                matches!(
                    kind(tree, *at),
                    Kind::Node(Node::Name) | Kind::Extra(Extra::Identifier)
                ) && name_token(tree, *at).is_some()
            }),
    )
}
