//! The `assignments` service: free functions named by verb over the domain's types, with no IO and no ports.

use ryo::languages::{Position, Tree};

use super::types::Operator;
use crate::domain::nodes::types::GoNode;
use crate::domain::syntax::service::{kind, nodes, tokens};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// Whether `token` is the operator of an assignment: `=`, `:=` or an operator with `=`.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "an assignment is told by its operator; every other token is not one"
)]
fn assigns(token: Token) -> Option<Token> {
    match token {
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
        | Token::AndNotAssign => Some(token),
        _ => None,
    }
}

/// The operator of an assignment: its place among the tokens of the element, and which it is.
#[must_use]
pub(crate) fn operator(tree: &Tree<'_>, at: Position) -> Option<Operator> {
    tokens(tree, at)
        .find_map(|each| assigns(each.token).map(|token| Operator { at: each.at, token }))
}

/// The elements before the operator of the assignment, the short variable declaration or the `range` clause at `at`: what it assigns to.
/// A `range` clause with no operator before its `range` assigns to nothing.
pub(crate) fn left<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    let keyword = (kind(tree, at) == Kind::Node(GoNode::RangeStmt))
        .then(|| tokens(tree, at).find(|each| each.token == Token::Range))
        .flatten()
        .map(|each| each.at);
    let stop = operator(tree, at)
        .map(|each| each.at)
        .filter(|place| keyword.is_none_or(|range| *place < range));
    nodes(tree, at).take_while(move |each| stop.is_some_and(|stop| *each < stop))
}

/// The elements after the operator of the assignment at `at`: what it assigns.
pub(crate) fn right<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    let stop = operator(tree, at).map(|each| each.at);
    nodes(tree, at).filter(move |each| stop.is_some_and(|stop| *each > stop))
}
