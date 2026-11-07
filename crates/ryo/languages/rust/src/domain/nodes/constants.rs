//! The names of the `nodes` concept.

/// The standard the node names come from, and its version: the `SyntaxKind` list of rust-analyzer's parser, as the pinned version of the library lists it.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STANDARD: &str = "ra_ap_parser 0.0.347";
