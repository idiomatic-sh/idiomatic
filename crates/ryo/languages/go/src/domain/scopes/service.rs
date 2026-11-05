//! The `scopes` service: free functions named by verb over the domain's types, with no IO and no ports. A scope is a function, a
//! method or a function literal; the file is the scope of everything outside them.

use ryo::languages::{self, Position, Tree};

use super::types::Scopeness;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{is_root, kind, parent};
use crate::domain::syntax::types::{Kind, Rootness};

/// Whether `kind` opens a scope of its own.
#[must_use]
pub(crate) const fn scopeness(kind: Kind) -> Scopeness {
    if matches!(kind, Kind::Node(Node::FuncDecl | Node::FuncLit)) {
        Scopeness::Scope
    } else {
        Scopeness::Inner
    }
}

/// The scope the element at `at` is in: the nearest function or function literal that holds it, or the root.
#[must_use]
pub(crate) fn enclosing(tree: &Tree<'_>, at: Position) -> Position {
    let mut above = parent(tree, at);
    while is_root(tree, above) == Rootness::Inner
        && scopeness(kind(tree, above)) == Scopeness::Inner
    {
        above = parent(tree, above);
    }
    above
}

/// The scope `at` is in, as a node id: the owner a row names.
#[must_use]
pub(crate) fn owner(tree: &Tree<'_>, at: Position) -> languages::NodeId {
    languages::node_id(tree.file, enclosing(tree, at))
}

/// The scope that encloses the scope at `at`, as a node id: the file encloses itself.
#[must_use]
pub(crate) fn scope_owner(tree: &Tree<'_>, at: Position) -> languages::NodeId {
    if is_root(tree, at) == Rootness::Root {
        languages::node_id(tree.file, at)
    } else {
        owner(tree, at)
    }
}
