//! The names of the `tokens` concept.

/// The standard the token names come from, and its version: the `SyntaxKind` list of rust-analyzer's parser, as the pinned version of the library lists it.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STANDARD: &str = crate::domain::nodes::constants::STANDARD;
