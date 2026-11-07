//! The failures of `query describe`.

/// Why `describe` failed: it cannot, the grammar is the engine's own, but every entry names its error.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DescribeError {}
