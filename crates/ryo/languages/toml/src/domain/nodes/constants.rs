//! The names of the `nodes` concept.

/// The standard the node names come from, and its version: the rules of the ABNF the TOML project publishes for the version of the
/// language the parser reads.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STANDARD: &str = "toml.abnf 1.1.0";
