//! The types of the `globset` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

/// A pattern compiled to match paths.
#[derive(Clone, Debug)]
pub(crate) struct Matcher {
    /// The compiled pattern.
    pub compiled: globset::GlobMatcher,
}
