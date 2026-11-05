//! The `targets` service: free functions named by verb over the domain's types, with no IO and no ports. It finds the token on which the adapter wrote what an
//! `@import` imports and what a `url()` points at, and what each belongs to: the nearest declaration or rule around it, or the
//! stylesheet.

use ryo::languages::{self, Position, Tree};

use super::constants::NAMESPACE;
use super::types::Url;
use crate::domain::nodes::service::category;
use crate::domain::nodes::types::{Category, Node};
use crate::domain::syntax::service::{is_root, kind, kind_bits, parent};
use crate::domain::syntax::types::{Kind, Rootness};
use crate::domain::tokens::types::Token;

/// The text the element at `at` spans, as it is written.
fn written<'a>(tree: &Tree<'a>, at: Position) -> Url<'a> {
    let (start, end) = (
        usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX),
        usize::try_from(languages::end_at(tree.elements, at).0).unwrap_or(usize::MAX),
    );
    tree.source
        .0
        .get(start..end)
        .and_then(|bytes| std::str::from_utf8(bytes).ok()) // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
        .map_or(Url(Default::default()), Url)
}

/// The token the adapter wrote a url's resolved target on: a url-token, or the function-token of a `url("..")` (`at` is the
/// function).
#[must_use]
pub(crate) fn url_token(tree: &Tree<'_>, at: Position) -> Option<Position> {
    let found = kind(tree, at);
    let token = if found == Kind::Token(Token::Url) {
        at
    } else if found == Kind::Node(Node::Function) {
        languages::child_of_kind(tree.elements, at, kind_bits(Kind::Token(Token::Function)))?
    } else {
        return None;
    };
    languages::decoded_at(tree.elements, token).map(|_| token)
}

/// The token the adapter wrote what the `@import` at `at` imports on: its at-keyword-token.
#[must_use]
pub(crate) fn import_token(tree: &Tree<'_>, at: Position) -> Option<Position> {
    if kind(tree, at) != Kind::Node(Node::AtRule) {
        return None;
    }
    let keyword =
        languages::child_of_kind(tree.elements, at, kind_bits(Kind::Token(Token::AtKeyword)))?;
    languages::decoded_at(tree.elements, keyword).map(|_| keyword)
}

/// Whether the url at `at` is a reference to a file: one outside the prelude of an `@import`, which `static_import` reads, and of an
/// `@namespace`, whose url names a namespace. The url itself when it is.
#[must_use]
pub(crate) fn reference(tree: &Tree<'_>, at: Position) -> Option<Position> {
    let mut place = parent(tree, at);
    while kind(tree, place) != Kind::Node(Node::AtRule) && is_root(tree, place) == Rootness::Inner {
        place = parent(tree, place);
    }
    let keyword = (kind(tree, place) == Kind::Node(Node::AtRule))
        .then(|| {
            languages::child_of_kind(
                tree.elements,
                place,
                kind_bits(Kind::Token(Token::AtKeyword)),
            )
        })
        .flatten();
    let excluded = keyword.is_some_and(|keyword| {
        import_token(tree, place).is_some()
            || written(tree, keyword).0.eq_ignore_ascii_case(NAMESPACE)
    });
    if excluded { None } else { Some(at) }
}

/// What the element at `at` belongs to: the nearest declaration or rule around it, or the stylesheet; never a component value.
#[must_use]
pub(crate) fn owner(tree: &Tree<'_>, at: Position) -> Position {
    let mut place = parent(tree, at);
    while matches!(kind(tree, place), Kind::Node(node) if category(node) == Category::ComponentValue)
        && is_root(tree, place) == Rootness::Inner
    {
        place = parent(tree, place);
    }
    place
}

/// Whether the string at `at` is an import specifier: the first string of an `@import`'s prelude, written bare or in `url("..")`, which
/// names the stylesheet it imports.
#[must_use]
pub(crate) fn specifier(tree: &Tree<'_>, at: Position) -> languages::Flag {
    let rule = owner(tree, at);
    let first =
        (rule.0 + 1..at.0).all(|place| kind(tree, Position(place)) != Kind::Token(Token::String));
    if import_token(tree, rule).is_some() && first {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}
