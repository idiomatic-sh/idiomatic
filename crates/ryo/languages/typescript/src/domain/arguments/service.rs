//! The `arguments` service: free functions named by verb that read the arguments a call writes, with no IO and no ports.

use crate::domain::names::service::{identifier, nodes};
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::{service::kind, types::Kind};
use ryo::languages::{self, Position, Symbol, Symbols, Tree};

/// The arguments the call or `new` at `call` writes, in order; the callee and the type arguments are no argument.
pub(crate) fn arguments_of<'a>(
    tree: &'a Tree<'_>,
    call: Position,
) -> impl Iterator<Item = Position> + 'a {
    nodes(tree, call)
        .skip(1)
        .filter(move |each| kind(tree, *each) != Kind::Node(TypeScriptNode::TSTypeParameterInstantiation))
}

/// The name the argument at `argument` hands on: the one identifier it is, behind a spread; the empty name when it is anything else.
#[must_use]
pub(crate) fn passed(tree: &Tree<'_>, symbols: &mut Symbols, argument: Position) -> Symbol {
    let mut current = argument;
    while kind(tree, current) == Kind::Node(TypeScriptNode::SpreadElement) {
        match nodes(tree, current).next() {
            Some(inner) => current = inner,
            None => return languages::NO_SYMBOL,
        }
    }
    if kind(tree, current) == Kind::Node(TypeScriptNode::Identifier) {
        identifier(tree, symbols, current)
    } else {
        languages::NO_SYMBOL
    }
}
