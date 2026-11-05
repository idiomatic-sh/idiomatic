//! The `visibilities` service: free functions named by verb that read the declarations a visibility is written on, each by the node
//! its `defines` row is known by, with no IO and no ports.

use super::types::Declaring;
use crate::domain::declarations::service::{imported, named_kind};
use crate::domain::nodes::types::RustNode;
use crate::domain::syntax::service::{child, end, kind, parent, start};
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, Tree};

/// The declarations the visibility at `at` is written on, each by the node its `defines` row is known by: the item, field or variant
/// that holds it, or, for a `use`, each name its tree binds.
pub(crate) fn visible<'a>(
    tree: &'a Tree<'a>,
    at: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    let holder = parent(tree, at);
    let element = kind(tree, holder);
    let used = element == Kind::Node(RustNode::Use);
    let until = end(tree, holder);
    let leaves = languages::positions(tree.elements)
        .skip(if used {
            usize::try_from(holder.0)
                .unwrap_or(usize::MAX)
                .saturating_add(1)
        } else {
            usize::MAX
        })
        .take_while(move |each| start(tree, *each) < until)
        .filter(move |each| {
            kind(tree, *each) == Kind::Node(RustNode::UseTree) && imported(tree, *each).is_some()
        });
    let own = (element != Kind::Node(RustNode::Use) && declares(tree, holder) == Declaring::Declares)
        .then_some(holder);
    own.into_iter().chain(leaves)
}

/// Whether the node at `at` is an item, a field or a variant that `defines` has a row for, known by its own node.
fn declares(tree: &Tree<'_>, at: languages::Position) -> Declaring {
    let element = kind(tree, at);
    let named = named_kind(element).is_some() || element == Kind::Node(RustNode::Fn);
    let declares = if named {
        child(tree, at, Kind::Node(RustNode::Name)).is_some()
    } else {
        element == Kind::Node(RustNode::TupleField)
            || (element == Kind::Node(RustNode::ExternCrate)
                && (child(tree, at, Kind::Node(RustNode::Rename)).is_some()
                    || child(tree, at, Kind::Node(RustNode::NameRef)).is_some()))
    };
    if declares {
        Declaring::Declares
    } else {
        Declaring::Silent
    }
}
