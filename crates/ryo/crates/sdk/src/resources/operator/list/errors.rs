//! The failures of `operator list`.

/// Why `list` failed: it cannot, the operators are the engine's own, but every entry names its error.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ListError {}
