//! The `scopes` service: the lexical environments ECMA-262 creates (the file, a function, a class, a block, a `catch`, a `for`
//! head and a class static block), and the owner of an occurrence, the nearest of them around it.

use super::types::Scope;
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::syntax::service::{kind, parent};
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, Position, Tree};

/// Whether a node of kind `kind` opens a lexical environment.
#[must_use]
pub(crate) fn opens(kind: Kind) -> Scope {
    if matches!(
        kind,
        Kind::Node(
            JavaScriptNode::Program
                | JavaScriptNode::FunctionDeclaration
                | JavaScriptNode::FunctionExpression
                | JavaScriptNode::ArrowFunctionExpression
                | JavaScriptNode::ClassDeclaration
                | JavaScriptNode::ClassExpression
                | JavaScriptNode::BlockStatement
                | JavaScriptNode::CatchClause
                | JavaScriptNode::ForStatement
                | JavaScriptNode::ForInStatement
                | JavaScriptNode::ForOfStatement
                | JavaScriptNode::StaticBlock
        )
    ) {
        Scope::Opens
    } else {
        Scope::Continues
    }
}

/// The nearest environment around the element at `at`; the file is its own.
#[must_use]
pub(crate) fn enclosing(tree: &Tree<'_>, at: Position) -> Position {
    let mut above = parent(tree, at);
    while above != parent(tree, above) && opens(kind(tree, above)) == Scope::Continues {
        above = parent(tree, above);
    }
    above
}

/// The owner of the occurrence at `at`, by its node: the environment around it.
#[must_use]
pub(crate) fn owner(tree: &Tree<'_>, at: Position) -> languages::NodeId {
    languages::node_id(tree.file, enclosing(tree, at))
}
