//! The role of a child, decided while the numbered table is written: the same rules as `service::field_of`, over the children of one
//! parent held in plain slices instead of over a finished tree. The writer already holds the whole tree of pieces when it writes, so
//! every sibling, before or after, is known when a child is entered.


mod holding;
mod placing;

use super::types::{Field, Returning};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;

/// The children of one parent, in source order: what the rules read to give each its role.
#[derive(Debug)]
pub(crate) struct Family<'a> {
    /// The kind of the parent.
    pub holder: Node,
    /// The kinds of its children.
    pub kinds: &'a [Kind],
    /// Where each child starts.
    pub starts: &'a [u32],
    /// Where each child ends.
    pub ends: &'a [u32],
    /// The file's text, which the spellings of tokens are cut from.
    pub source: &'a str,
}

/// The role of the child `at` under its parent; none for a token that spells no operator and for an extra.
#[must_use]
pub(crate) fn field_of_child(fam: &Family<'_>, at: usize) -> Field {
    let child = fam.kinds[at];
    match child {
        Kind::Token(_) if placing::operator_of(fam) == Some(at) => Field::Operator,
        Kind::Token(_) | Kind::Extra(_) | Kind::Pattern(_) => Field::None,
        Kind::Node(Node::Decorator) => Field::Decorators,
        Kind::Node(Node::TSTypeAnnotation) if placing::returning(fam.holder) == Returning::Returns => Field::ReturnType,
        Kind::Node(Node::TSTypeAnnotation) => Field::TypeAnnotation,
        Kind::Node(Node::TSTypeParameterDeclaration) => Field::TypeParameters,
        Kind::Node(Node::TSTypeParameterInstantiation)
            if matches!(fam.holder, Node::ClassDeclaration | Node::ClassExpression) =>
        {
            Field::SuperTypeArguments
        }
        Kind::Node(Node::TSTypeParameterInstantiation) => Field::TypeArguments,
        Kind::Node(_) => holding::held(fam, at, fam.holder, child),
    }
}
