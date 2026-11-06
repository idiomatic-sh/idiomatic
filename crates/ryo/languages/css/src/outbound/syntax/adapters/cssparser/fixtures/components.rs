//! Part of the walk of the `cssparser` fixtures: contents, Depth, Piece and more.

use cssparser::{ParseError, Parser, Token};
use super::super::types::{ByteIndex, Contents};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::service::mirror;
use super::tokens::{is_matcher, is_trivia, opens, place, token_kind};







/// Whether the value `input` holds has a `{}` block at its top beside other content.
pub(super) fn contents(input: &mut Parser<'_>) -> Contents {
    let (mut block, mut content) = (false, 0_usize);
    while let Ok(token) = input.next() {
        block |= matches!(token, Token::CurlyBracketBlock);
        content += 1;
    }
    if block && content > 1 {
        Contents::HoldsBlock
    } else {
        Contents::Plain
    }
}

/// How deep a piece nests among pieces that start at the same place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Depth {
    /// A rule.
    Rule,
    /// A declaration.
    Declaration,
    /// A function or a simple block.
    Component,
    /// A token.
    Token,
}

/// A piece of the tree before it is nested: its kind, where it lies, and how it nests.
#[derive(Clone, Copy, Debug)]
pub(super) struct Piece {
    /// What it is: a node or a token of the stylesheet.
    pub(super) kind: Kind,
    /// Where it starts in the text read.
    pub(super) start: ByteIndex,
    /// Where it ends in the text read.
    pub(super) end: ByteIndex,
    /// How deep it nests, which orders two pieces that start together.
    pub(super) depth: Depth,
}

/// Every token, function and simple block of `parser`'s text, from the library's tokenizer and its nested blocks.
pub(super) fn components(parser: &mut Parser<'_>, pieces_out: &mut Vec<Piece>) {
    loop {
        let start = place(parser);
        let Ok(token) = parser.next_including_whitespace_and_comments().cloned() else {
            return;
        };
        let end = place(parser);
        let kind = token_kind(&token);
        let opened = opens(&token);
        let node = pieces_out.len();
        if let Some(opened) = opened {
            pieces_out.push(Piece {
                kind: Kind::Node(opened),
                start,
                end,
                depth: Depth::Component,
            });
        }
        if is_matcher(&token) {
            let middle = ByteIndex(start.0 + 1);
            pieces_out.push(token_piece(kind, start, middle));
            pieces_out.push(token_piece(kind, middle, end));
        } else {
            pieces_out.push(token_piece(kind, start, end));
        }
        if opened.is_some() {
            let before = pieces_out.len();
            let nested: Result<(), ParseError<()>> = parser.parse_nested_block(|inner| {
                components(inner, pieces_out);
                Ok(())
            });
            let after = place(parser);
            let inside = pieces_out
                .get(before..)
                .unwrap_or_default()
                .iter()
                .map(|each| each.end)
                .max()
                .unwrap_or(end);
            match (nested, kind) {
                (Ok(()), Kind::Token(lexeme)) if after > inside => {
                    if let Some(closing) = mirror(lexeme) {
                        pieces_out.push(token_piece(Kind::Token(closing), inside, after));
                    }
                }
                (Err(_), _) if after > end => {
                    pieces_out.push(token_piece(Kind::Extra(Extra::Error), end, after));
                }
                _ => {}
            }
            if let Some(piece) = pieces_out.get_mut(node) {
                piece.end = after;
            }
        }
    }
}

/// A token piece.
pub(super) const fn token_piece(kind: Kind, start: ByteIndex, end: ByteIndex) -> Piece {
    Piece {
        kind,
        start,
        end,
        depth: Depth::Token,
    }
}

/// Where a run of component values ends: after its last one that is not white space or a comment; `from` for none.
pub(super) fn content_end(input: &mut Parser<'_>, from: ByteIndex) -> ByteIndex {
    let mut end = from;
    loop {
        let Ok(token) = input.next_including_whitespace_and_comments() else {
            return end;
        };
        if is_trivia(token) {
            continue;
        }
        if opens(token).is_some() {
            // The block is read only to pass over it: whether its contents parse does not move where the run ends.
            drop(input.parse_nested_block(|inner| {
                skip_content(inner);
                Ok::<(), ParseError<()>>(())
            }));
        }
        end = place(input);
    }
}

/// Passes over the rest of a run of component values, nested blocks included.
pub(super) fn skip_content(input: &mut Parser<'_>) {
    while input.next_including_whitespace_and_comments().is_ok() {}
}
