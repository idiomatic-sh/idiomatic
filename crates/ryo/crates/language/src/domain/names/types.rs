//! The types of the `names` concept: where a name being made begins.

/// Where a name being made begins in the derived text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameStart(pub usize);
