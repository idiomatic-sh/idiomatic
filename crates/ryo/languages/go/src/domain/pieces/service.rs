//! The `pieces` service: free functions named by verb over the pieces found, with no IO and no ports.

use ryo::languages::NameText;

use super::types::{DecodedAt, Lexeme, Order, Pieces, Placed};
use crate::domain::grammar::service::{ending, start};
use crate::domain::grammar::types::{ByteIndex, Concrete, Index};
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

/// Finds a node of `go/ast` over the bytes `start..end`.
pub(crate) fn node(pieces: &mut Pieces, kind: Node, from: ByteIndex, to: ByteIndex) {
    let order = Order(u32::try_from(pieces.nodes.len()).unwrap_or(u32::MAX));
    pieces.nodes.push(Placed {
        kind,
        start: from,
        end: to,
        order,
    });
}

/// Finds a node of `go/ast` over a node of the library's tree.
pub(crate) fn over(pieces: &mut Pieces, tree: &Concrete<'_>, kind: Node, at: Index) {
    node(pieces, kind, start(tree, at), ending(tree, at));
}

/// Finds a token over the bytes `start..end`.
pub(crate) fn token(pieces: &mut Pieces, kind: Token, from: ByteIndex, to: ByteIndex) {
    pieces.lexemes.push(Lexeme {
        kind,
        start: from,
        end: to,
        decoded: None,
    });
}

/// Finds a string token over the bytes `start..end`, with the text it stands for.
pub(crate) fn string(pieces: &mut Pieces, from: ByteIndex, to: ByteIndex, value: NameText<'_>) {
    let before = ByteIndex(pieces.decoded.0.len());
    pieces.decoded.0.push_str(value.0);
    pieces.lexemes.push(Lexeme {
        kind: Token::String,
        start: from,
        end: to,
        decoded: Some(DecodedAt {
            from: before,
            to: ByteIndex(pieces.decoded.0.len()),
        }),
    });
}

/// The text a string token stands for.
#[must_use]
pub(crate) fn value_of<'a>(pieces: &'a Pieces, lexeme: &Lexeme) -> Option<NameText<'a>> {
    let at = lexeme.decoded?;
    pieces.decoded.0.get(at.from.0..at.to.0).map(NameText)
}

/// Orders the nodes as the file does: a node that starts earlier, or from the same place ends later, holds the other; of two over the
/// same bytes, the one found first holds the other.
pub(crate) fn arrange(pieces: &mut Pieces) {
    pieces
        .nodes
        .sort_unstable_by_key(|each| (each.start, std::cmp::Reverse(each.end), each.order));
}
