//! The names of the `tokens` concept.

/// The standard the token names come from, and its version: the terminal rules of the ABNF the TOML project publishes for the version of the
/// language the parser reads.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STANDARD: &str = crate::domain::nodes::constants::STANDARD;
