//! The `attribute_paths` service: free functions named by verb that find the paths a file's attributes write and the name each is
//! written as, with no IO and no ports. Nothing here allocates: a path is found by stepping from sibling to sibling, and its name
//! is pushed into the symbols token by token.

use ryo::languages::{self, Position, Tree};

use super::types::{WrittenPath, WrittenPaths};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{end, is_inside, is_token, kind, last_descendant, parent, start};
use crate::domain::syntax::types::{Containment, Kind, Leaf};
use crate::domain::tokens::types::Token;

/// The paths the element at `at` writes when it is part of an attribute: the path of a meta (the outermost path node, not one of
/// its qualifiers), or every path written among the tokens directly in a token tree, a run of segments joined by `::` with an
/// optional leading `::`. Any other element writes none.
#[must_use]
pub(crate) fn written_paths<'a>(tree: &'a Tree<'a>, at: Position) -> WrittenPaths<'a> {
    let none = WrittenPaths { tree, single: None, cursor: None, end: at };
    if is_inside(tree, at, Kind::Node(Node::Attr)) != Containment::Inside {
        return none;
    }
    match kind(tree, at) {
        Kind::Node(Node::Path) if kind(tree, parent(tree, at)) != Kind::Node(Node::Path) => WrittenPaths {
            single: Some(WrittenPath { first: at, last: at }),
            ..none
        },
        Kind::Node(Node::TokenTree) => WrittenPaths {
            cursor: languages::children(tree.elements, at).next(),
            end: last_descendant(tree, at),
            ..none
        },
        _ => none,
    }
}

impl Iterator for WrittenPaths<'_> {
    type Item = WrittenPath;

    fn next(&mut self) -> Option<WrittenPath> {
        if let Some(single) = self.single.take() {
            return Some(single);
        }
        loop {
            let first = significant(self.tree, self.cursor, self.end)?;
            match path_from(self.tree, first, self.end) {
                Some(path) => {
                    self.cursor = next_sibling(self.tree, path.last, self.end);
                    return Some(path);
                }
                None => self.cursor = next_sibling(self.tree, first, self.end),
            }
        }
    }
}

/// The name a path is written as: the text of each of its tokens but whitespace and comments, in order. A segment keeps the `r#`
/// it is written with; a token tree's `::`, two `:` tokens, reads `::`.
#[must_use]
pub(crate) fn path_name(tree: &Tree<'_>, symbols: &mut languages::Symbols, path: WrittenPath) -> languages::Symbol {
    let begun = languages::begin_name(symbols);
    for each in path.first.0..=last_descendant(tree, path.last).0 {
        let each = Position(each);
        if is_token(tree, each) == Leaf::Token && !is_trivia(tree, each) {
            languages::push_span(symbols, tree.source, start(tree, each), end(tree, each));
        }
    }
    languages::end_name(symbols, tree.source, begun)
}

/// The path that starts at `first` among a token tree's children, when one does: an optional `::`, a segment, then `::` and a
/// segment as long as they follow.
fn path_from(tree: &Tree<'_>, first: Position, end: Position) -> Option<WrittenPath> {
    let segment = match separator_end(tree, first, end) {
        Some(second) => significant_after(tree, second, end)?,
        None => first,
    };
    if !is_segment(tree, segment) {
        return None;
    }
    let mut last = segment;
    while let Some(next) = significant_after(tree, last, end)
        .and_then(|colon| separator_end(tree, colon, end))
        .and_then(|second| significant_after(tree, second, end))
        .filter(|each| is_segment(tree, *each))
    {
        last = next;
    }
    Some(WrittenPath { first, last })
}

/// The second `:` of a `::` that starts at `at`, when one does.
fn separator_end(tree: &Tree<'_>, at: Position, end: Position) -> Option<Position> {
    let colon = |each: &Position| kind(tree, *each) == Kind::Token(Token::Colon);
    Some(at).filter(colon)?;
    significant_after(tree, at, end).filter(colon)
}

/// The first sibling from `from` on that is neither whitespace nor a comment.
fn significant(tree: &Tree<'_>, from: Option<Position>, end: Position) -> Option<Position> {
    let mut at = from;
    while let Some(each) = at {
        if !is_trivia(tree, each) {
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

/// Whether the element at `at` is whitespace or a comment.
fn is_trivia(tree: &Tree<'_>, at: Position) -> bool {
    matches!(kind(tree, at), Kind::Token(Token::Whitespace | Token::Comment))
}

/// Whether the token at `at` can be a path segment: an identifier, `self`, `super`, `crate` or `Self`.
fn is_segment(tree: &Tree<'_>, at: Position) -> bool {
    matches!(
        kind(tree, at),
        Kind::Token(Token::Ident | Token::SelfKw | Token::SuperKw | Token::CrateKw | Token::SelfTypeKw)
    )
}
