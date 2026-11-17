//! The text the `paths` service reads a path by.

/// The separator of a path's segments.
pub const SEPARATOR: &str = "/";

/// A segment that names the directory it is in.
pub const CURRENT: &str = ".";

/// A segment that climbs to the directory above.
pub const PARENT: &str = "..";

/// The text of the root, and of a path relative to itself.
pub const NOTHING: &str = "";
/// Why a path is refused when its name is not UTF-8: every path a rule reads, a definition names and a finding names is text.
pub const NOT_UTF8: &str = "the name is not UTF-8";
