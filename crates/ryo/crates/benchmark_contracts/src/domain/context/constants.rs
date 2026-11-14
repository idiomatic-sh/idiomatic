//! The names a sealed project context spells in its digest and its documents.
/// The entry state of a path sealed as absent, and the digest section that lists such paths.
pub const MISSING: &str = "missing";
/// The digest section that lists the sources a case lints.
pub const SOURCE: &str = "source";
/// The digest section that lists the auxiliary files a case supplies.
pub const AUXILIARY: &str = "auxiliary";
/// The tag the content seal of a listed project context starts with.
pub const LISTED_DISK_CONTEXT: &str = "listed-disk-context";
