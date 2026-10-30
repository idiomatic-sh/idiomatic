//! The `siblings` service: free functions named by verb that say where an element stands among the children of its parent, comments
//! and white space left out: how many of a kind come before it, whether it is the first or the last, and what is right before it.

use ryo::languages::{self, Count, Position, Tree};

use super::types::{Match, Standing, Triviality, Wanted};
use crate::domain::syntax::service::{is_token, kind, parent};
use crate::domain::syntax::types::{Kind, Leaf};
use crate::domain::tokens::types::Token;

/// Whether the element of kind `each` is trivia: a comment, white space or bytes the parser could not read, which no field holds.
#[must_use]
pub(crate) fn triviality(each: Kind) -> Triviality {
    if matches!(each, Kind::Token(Token::Comment) | Kind::Extra(_)) {
        Triviality::Trivia
    } else {
        Triviality::Significant
    }
}

/// Whether an element of kind `each` is one `wanted` counts.
#[must_use]
pub(crate) fn matching(wanted: Wanted, each: Kind) -> Match {
    let found = match wanted {
        Wanted::Node => matches!(each, Kind::Node(_)),
        Wanted::NodeOf(node) => each == Kind::Node(node),
        Wanted::Token(token) => each == Kind::Token(token),
        Wanted::Assigning => matches!(
            each,
            Kind::Token(
                Token::Assign
                    | Token::Define
                    | Token::AddAssign
                    | Token::SubAssign
                    | Token::MulAssign
                    | Token::QuoAssign
                    | Token::RemAssign
                    | Token::AndAssign
                    | Token::OrAssign
                    | Token::XorAssign
                    | Token::ShlAssign
                    | Token::ShrAssign
                    | Token::AndNotAssign
            )
        ),
    };
    if found { Match::Wanted } else { Match::Other }
}

/// The children of the parent of `at` that are no trivia, in order.
pub(crate) fn siblings<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Position> + 'a {
    languages::children(tree.elements, parent(tree, at))
        .filter(move |each| triviality(kind(tree, *each)) == Triviality::Significant)
}

/// How many siblings of `at` before it `wanted` counts.
#[must_use]
pub(crate) fn before(tree: &Tree<'_>, at: Position, wanted: Wanted) -> Count {
    Count(
        siblings(tree, at)
            .take_while(|each| *each != at)
            .filter(|each| matching(wanted, kind(tree, *each)) == Match::Wanted)
            .count(),
    )
}

/// How many siblings of `at`, before it, after it or itself, `wanted` counts.
#[must_use]
pub(crate) fn among(tree: &Tree<'_>, at: Position, wanted: Wanted) -> Count {
    Count(
        siblings(tree, at)
            .filter(|each| matching(wanted, kind(tree, *each)) == Match::Wanted)
            .count(),
    )
}

/// How many siblings of `at` after it are nodes.
#[must_use]
pub(crate) fn nodes_after(tree: &Tree<'_>, at: Position) -> Count {
    Count(
        siblings(tree, at)
            .skip_while(|each| *each != at)
            .skip(1)
            .filter(|each| is_token(tree, *each) == Leaf::Node)
            .count(),
    )
}

/// The sibling of `at` right before it, trivia left out.
#[must_use]
pub(crate) fn previous(tree: &Tree<'_>, at: Position) -> Option<Kind> {
    siblings(tree, at)
        .take_while(|each| *each != at)
        .last()
        .map(|each| kind(tree, each))
}

/// Whether `at` is the first sibling, trivia left out.
#[must_use]
pub(crate) fn first(tree: &Tree<'_>, at: Position) -> Standing {
    if siblings(tree, at).next() == Some(at) {
        Standing::Edge
    } else {
        Standing::Inside
    }
}

/// Whether `at` is the last sibling, trivia left out.
#[must_use]
pub(crate) fn last(tree: &Tree<'_>, at: Position) -> Standing {
    if siblings(tree, at).last() == Some(at) {
        Standing::Edge
    } else {
        Standing::Inside
    }
}

/// The bracket the children of `list` open with, trivia left out, when they open with one: a field list of a result alone has none.
#[must_use]
pub(crate) fn opening(tree: &Tree<'_>, list: Position) -> Option<Token> {
    let first = languages::children(tree.elements, list)
        .find(|each| triviality(kind(tree, *each)) == Triviality::Significant)?;
    match kind(tree, first) {
        Kind::Token(spelled @ (Token::Lparen | Token::Lbrace | Token::Lbrack)) => Some(spelled),
        Kind::Token(_) | Kind::Node(_) | Kind::Extra(_) => None,
    }
}
