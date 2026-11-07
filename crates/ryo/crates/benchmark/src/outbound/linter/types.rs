//! Plain data of the linter port.

/// The canonical text of one identity input of a reference process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalText(pub String);
