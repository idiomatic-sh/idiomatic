//! The `resolutions` service: free functions named by verb that read the type name a type reference writes and what holds it, with
//! no IO and no ports.

use crate::domain::names::service::nodes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{
    service::{kind, parent},
    types::Kind,
};
use ryo::languages::{self, Position, Tree};

/// The one name the type reference at `at` writes: a reference to an identifier, with no qualifier before it. The identifier is the
/// first node of the reference.
#[must_use]
pub(crate) fn plain_type_name(tree: &Tree<'_>, at: Position) -> Option<Position> {
    if kind(tree, at) != Kind::Node(Node::TSTypeReference) {
        return None;
    }
    nodes(tree, at)
        .next()
        .filter(|name| kind(tree, *name) == Kind::Node(Node::Identifier))
}

/// What holds the element at `at`: the nearest class, interface, type alias, enum, function or method it is written in, else the file.
#[must_use]
pub(crate) fn holder(tree: &Tree<'_>, at: Position) -> languages::NodeId {
    let mut above = at;
    while parent(tree, above) != above {
        above = parent(tree, above);
        if matches!(
            kind(tree, above),
            Kind::Node(
                Node::ClassDeclaration
                    | Node::TSInterfaceDeclaration
                    | Node::TSTypeAliasDeclaration
                    | Node::TSEnumDeclaration
                    | Node::FunctionDeclaration
                    | Node::MethodDefinition
            )
        ) {
            return languages::node_id(tree.file, above);
        }
    }
    languages::node_id(tree.file, above)
}
