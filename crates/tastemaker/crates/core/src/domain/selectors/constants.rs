//! The text the `selectors` concept spells patterns with.

/// The glob that passes every path, which a regex selector spells its glob with: the expression decides.
pub const EVERY_PATH: &str = "**";
/// The key of a regex selector's table.
pub const REGEX_KEY: &str = "regex";
/// The key of a part selector's table.
pub const PART_KEY: &str = "part";
/// The key that makes a part selector reach any depth.
pub const RECURSIVE_KEY: &str = "recursive";
/// The key of a placed-by selector's table.
pub const PLACED_BY_KEY: &str = "placed-by";
/// The pattern that is the base directory itself.
pub const BASE_ITSELF: &str = ".";
/// The characters a regular expression gives a meaning, escaped where a pattern writes them as text.
pub const REGEX_SPECIALS: &str = "\\.+*?()|[]{}^$#&-~";
/// A pattern's segment of any depth, and the glob it is spelled as alone.
pub const ANY_DEPTH: &str = "**";
/// The glob of a leading `**`.
pub const ANY_DEPTH_LEADING: &str = "**/";
/// The glob of a trailing `**`.
pub const ANY_DEPTH_TRAILING: &str = "/**";
/// The glob of a `**` between segments.
pub const ANY_DEPTH_MIDDLE: &str = "/**/";
/// The expression of `**` alone: every path.
pub const WHOLE_DEPTH: &str = "[^/]+(?:/[^/]+)*";
/// The expression of a leading `**`: any directories, or none.
pub const WHOLE_DEPTH_LEADING: &str = "(?:[^/]+/)*";
/// The expression of a trailing `**`: one segment or more.
pub const WHOLE_DEPTH_TRAILING: &str = "(?:/[^/]+)+";
/// The expression of a `**` between segments.
pub const WHOLE_DEPTH_MIDDLE: &str = "/(?:[^/]+/)*";
/// The expression of a `{*name}` capture: one or more whole segments.
pub const CAPTURE_SEGMENTS: &str = "([^/]+(?:/[^/]+)*)";
/// The expression of a `{name}` capture within a segment: never empty.
pub const CAPTURE_SEGMENT: &str = "([^/]+)";
/// The expression of a `*` within a segment.
pub const WITHIN_SEGMENT: &str = "[^/]*";
/// The expression of a `?`: one character.
pub const ONE_CHARACTER: &str = "[^/]";
