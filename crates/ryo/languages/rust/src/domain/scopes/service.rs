//! The `scopes` service: free functions named by verb that say what encloses an element, with no
//! IO and no ports. The file's module, a nested module, an `impl`, a trait and a function are scopes; what no scope encloses belongs to
//! the file's module.

use super::types::Namelessness;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{ancestors, child, kind};
use crate::domain::syntax::types::Kind;
use languages::Tree;
use ryo::languages;

use super::constants::FILE_OWNER;

/// The nearest function that encloses the element at `at`, itself not counted.
#[must_use]
pub(crate) fn enclosing_function(
    tree: &Tree<'_>,
    at: languages::Position,
) -> Option<languages::Position> {
    ancestors(tree, at).find(|each| kind(tree, *each) == Kind::Node(Node::Fn))
}

/// The scope that directly encloses the scope at `at`: the nearest function, `impl`, trait or module that holds it, or the module of the
/// file, which is the root and encloses itself.
#[must_use]
pub(crate) fn enclosing_scope(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    ancestors(tree, at)
        .find(|each| {
            matches!(
                kind(tree, *each),
                Kind::Node(Node::Fn | Node::Impl | Node::Trait | Node::Module)
            )
        })
        .unwrap_or(FILE_OWNER)
}

/// The owner of the element at `at`: the function that encloses it, or the file.
#[must_use]
pub(crate) fn owner(tree: &Tree<'_>, at: languages::Position) -> languages::NodeId {
    languages::node_id(
        tree.file,
        enclosing_function(tree, at).unwrap_or(FILE_OWNER),
    )
}

/// Whether the element at `at` is a `fn` that writes no name: what the parser makes of a broken one, which declares nothing.
#[must_use]
pub(crate) fn is_nameless_function(tree: &Tree<'_>, at: languages::Position) -> Namelessness {
    if kind(tree, at) == Kind::Node(Node::Fn) && child(tree, at, Kind::Node(Node::Name)).is_none() {
        Namelessness::Nameless
    } else {
        Namelessness::Named
    }
}
