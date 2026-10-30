//! The texts of the `tree_walk` concept.

/// What a read of the remembered nearest ancestors says when it is given an element they hold no slot for.
pub(super) const SLOT_OF_ELEMENT: &str = "a slot for each element of the tree the walk reads";

/// Not known yet, for the nearest ancestor of an element.
pub(super) const UNKNOWN: u32 = u32::MAX - 1;

/// Known to be none, for the nearest ancestor of an element.
pub(super) const NONE: u32 = u32::MAX;
