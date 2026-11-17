//! The `attribute_paths` service: free functions named by verb that find the paths a file's attributes write and the name each is
//! written as, with no IO and no ports. Nothing here allocates: a path is found by stepping from sibling to sibling, and its name
//! is pushed into the symbols token by token.

use ryo::languages::{self, Position, Tree};

use super::types::{PathExtent, Segmenthood, Significance};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{
    end, is_inside, is_token, kind, last_descendant, parent, start,
};
use crate::domain::syntax::types::{Containment, Kind, Leaf};
use crate::domain::tokens::types::Token;

/// The paths the element at `at` writes when it is part of an attribute: the path of a meta (the outermost path node, not one of
/// its qualifiers), or every path written among the tokens directly in a token tree, a run of segments joined by `::` with an
/// optional leading `::`. Any other element writes none.
pub(crate) fn written_paths<'a>(
    tree: &'a Tree<'a>,
    at: Position,
) -> impl Iterator<Item = PathExtent> + 'a {
    let inside = is_inside(tree, at, Kind::Node(Node::Attr)) == Containment::Inside;
    let mut single = (inside
        && kind(tree, at) == Kind::Node(Node::Path)
        && kind(tree, parent(tree, at)) != Kind::Node(Node::Path))
    .then_some(PathExtent {
        first: at,
        last: at,
    });
    let mut cursor = (inside && kind(tree, at) == Kind::Node(Node::TokenTree))
        .then(|| languages::children(tree.elements, at).next())
        .flatten();
    let end = last_descendant(tree, at);
    std::iter::from_fn(move || {
        if let Some(path) = single.take() {
            return Some(path);
        }
        loop {
            let first = significant(tree, cursor, end)?;
            match path_from(tree, first, end) {
                Some(path) => {
                    cursor = next_sibling(tree, path.last, end);
                    return Some(path);
                }
                None => cursor = next_sibling(tree, first, end),
            }
        }
    })
}

/// The name a path is written as: the text of each of its tokens but whitespace and comments, in order. A segment keeps the `r#`
/// it is written with; a token tree's `::`, two `:` tokens, reads `::`.
#[must_use]
pub(crate) fn path_name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    path: PathExtent,
) -> languages::Symbol {
    let begun = languages::begin_name(symbols);
    for each in path.first.0..=last_descendant(tree, path.last).0 {
        let each = Position(each);
        if is_token(tree, each) == Leaf::Token && significance(tree, each) == Significance::Read {
            languages::push_span(symbols, tree.source, start(tree, each), end(tree, each));
        }
    }
    languages::end_name(symbols, tree.source, begun)
}

/// The path that starts at `first` among a token tree's children, when one does: an optional `::`, a segment, then `::` and a
/// segment as long as they follow.
fn path_from(tree: &Tree<'_>, first: Position, end: Position) -> Option<PathExtent> {
    let segment = match separator_end(tree, first, end) {
        Some(second) => significant_after(tree, second, end)?,
        None => first,
    };
    if segmenthood(tree, segment) != Segmenthood::Segment {
        return None;
    }
    let mut last = segment;
    while let Some(next) = significant_after(tree, last, end)
        .and_then(|colon| separator_end(tree, colon, end))
        .and_then(|second| significant_after(tree, second, end))
        .filter(|each| segmenthood(tree, *each) == Segmenthood::Segment)
    {
        last = next;
    }
    Some(PathExtent { first, last })
}

/// The second `:` of a `::` that starts at `at`, when one does: the very next sibling, since rustc joins only colons that touch.
fn separator_end(tree: &Tree<'_>, at: Position, end: Position) -> Option<Position> {
    let colon = |each: &Position| kind(tree, *each) == Kind::Token(Token::Colon);
    Some(at)
        .filter(colon)
        .and_then(|first| next_sibling(tree, first, end))
        .filter(colon)
}

/// The first sibling from `from` on that is neither whitespace nor a comment.
fn significant(tree: &Tree<'_>, from: Option<Position>, end: Position) -> Option<Position> {
    let mut at = from;
    while let Some(each) = at {
        if significance(tree, each) == Significance::Read {
            return Some(each);
        }
        at = next_sibling(tree, each, end);
    }
    None
}

/// The first sibling after `at` that is neither whitespace nor a comment.
fn significant_after(tree: &Tree<'_>, at: Position, end: Position) -> Option<Position> {
    significant(tree, next_sibling(tree, at, end), end)
}

/// The sibling after `at`, within the token tree that ends at `end`.
fn next_sibling(tree: &Tree<'_>, at: Position, end: Position) -> Option<Position> {
    let next = Position(last_descendant(tree, at).0 + 1);
    (next.0 <= end.0).then_some(next)
}

/// Whether a path reads the element at `at` or skips it as whitespace or a comment.
fn significance(tree: &Tree<'_>, at: Position) -> Significance {
    if matches!(
        kind(tree, at),
        Kind::Token(Token::Whitespace | Token::Comment)
    ) {
        Significance::Skipped
    } else {
        Significance::Read
    }
}

/// Whether the token at `at` can be a path segment: an identifier, `self`, `super`, `crate` or `Self`.
fn segmenthood(tree: &Tree<'_>, at: Position) -> Segmenthood {
    if matches!(
        kind(tree, at),
        Kind::Token(
            Token::Ident | Token::SelfKw | Token::SuperKw | Token::CrateKw | Token::SelfTypeKw
        )
    ) {
        Segmenthood::Segment
    } else {
        Segmenthood::Other
    }
}
