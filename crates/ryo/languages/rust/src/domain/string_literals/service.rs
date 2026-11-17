//! The `string_literals` service: free functions named by verb that find the token a string literal is written with, with no IO
//! and no ports.

use ryo::languages::{self, Tree};

use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, is_inside, kind, parent};
use crate::domain::syntax::types::{Containment, Kind};
use crate::domain::tokens::types::Token;

/// The string token a string literal is written with: the `string` child of a literal expression, or a `string` token written
/// on its own where the language reads a value: in an attribute's arguments, in a `cfg` predicate and in an ABI. A string in
/// the token tree of any other macro is the macro's input, not a literal.
#[must_use]
pub(crate) fn string_token(
    tree: &Tree<'_>,
    at: languages::Position,
) -> Option<languages::Position> {
    let written = kind(tree, at);
    if written == Kind::Node(Node::Literal) {
        return child(tree, at, Kind::Token(Token::String));
    }
    if written != Kind::Token(Token::String) {
        return None;
    }
    let holder = kind(tree, parent(tree, at));
    let read = holder == Kind::Node(Node::CfgAtom)
        || holder == Kind::Node(Node::Abi)
        || (holder == Kind::Node(Node::TokenTree)
            && is_inside(tree, at, Kind::Node(Node::Attr)) == Containment::Inside);
    read.then_some(at)
}
