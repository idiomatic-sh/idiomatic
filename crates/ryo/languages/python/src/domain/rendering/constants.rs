//! The constants of the `rendering` concept.

/// What joins the rendered items of a tuple or a list.
pub(crate) const LIST_SEPARATOR: &str = ", ";

/// Why a node has no rendering: no expression of the tree, or a form whose embedding the tree does not prove.
pub(crate) const UNRENDERED: ryo::languages::UnknownReason = ryo::languages::UnknownReason(24);
