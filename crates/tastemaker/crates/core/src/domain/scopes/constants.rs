//! The text the `scopes` concept writes.

/// What a `#closed` finding on a file under a `rules-exclude` adds: the key keeps rules off the file, never the file out
/// of its placement.
pub const RULES_EXCLUDE_HELP: &str = "a rules-exclude keeps rules off a path; it does not exempt a file from being placed: place it with a template or spec entry (`tm add`), or remove it";
