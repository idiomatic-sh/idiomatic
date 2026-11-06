//! The constants of the part: the text its code uses.

/// The tag that opens the context identity, apart from the digests of other parts.
pub(crate) const IDENTITY_TAG: &[u8] = b"project-context";
/// What the identity records of a repository read from disk.
pub(crate) const DISK_REPOSITORY: &[u8] = b"disk";
/// What the identity records of a repository the caller gave.
pub(crate) const GIVEN_REPOSITORY: &[u8] = b"given";
