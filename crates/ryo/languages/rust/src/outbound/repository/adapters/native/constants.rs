//! The words the native repository adapter reads paths with.

/// The prefix of a verbatim Windows path, which `canonicalize` gives there.
pub(super) const VERBATIM: &str = r"\\?\";
