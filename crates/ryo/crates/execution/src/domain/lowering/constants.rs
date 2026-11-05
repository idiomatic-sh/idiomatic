//! The names of the `lowering` concept: the columns of the tree relations a lowered step reads.

use super::types::ElementPart;

/// The columns of the elements of a tree, in order.
pub(crate) const ELEMENT: [ElementPart; 5] = [
    ElementPart::Node,
    ElementPart::Kind,
    ElementPart::Text,
    ElementPart::File,
    ElementPart::Span,
];

/// The columns of the edges of a tree, in order.
pub(crate) const EDGE: [ElementPart; 5] = [
    ElementPart::Parent,
    ElementPart::Node,
    ElementPart::Ordinal,
    ElementPart::Role,
    ElementPart::File,
];
