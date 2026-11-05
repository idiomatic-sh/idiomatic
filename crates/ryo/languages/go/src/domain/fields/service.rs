//! The `fields` service: free functions named by verb over the domain's types, with no IO and no ports. A `Field` holds the names it
//! declares, then its type, then (in a struct) its tag; one with a single element has no name, and that element is its type.

use ryo::languages::{Position, Tree};

use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, nodes};
use crate::domain::syntax::types::Kind;

/// The type of the field at `field`: its last element, or the one before the tag.
#[must_use]
pub(crate) fn type_of(tree: &Tree<'_>, field: Position) -> Option<Position> {
    let (mut last, mut before) = (None, None);
    for each in nodes(tree, field) {
        before = last;
        last = Some(each);
    }
    match last {
        Some(tag) if kind(tree, tag) == Kind::Node(Node::BasicLit) && before.is_some() => before,
        other => other,
    }
}

/// The names the field at `field` declares, in order.
pub(crate) fn names<'a>(
    tree: &'a Tree<'_>,
    field: Position,
) -> impl Iterator<Item = Position> + 'a {
    let declared = type_of(tree, field);
    nodes(tree, field)
        .take_while(move |each| Some(*each) != declared)
        .filter(|each| kind(tree, *each) == Kind::Node(Node::Ident))
}
