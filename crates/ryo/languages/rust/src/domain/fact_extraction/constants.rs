//! The constants of the `fact_extraction` concept: the text its service looks for.

/// The name of the attribute that says where a module's file is.
pub(crate) const PATH_ATTRIBUTE: &str = "path";

/// The attribute that builds an item for the package's unit tests alone, its whitespace left out.
pub(crate) const TEST_ATTRIBUTE: &str = "#[cfg(test)]";
/// The visibility that exports an item to the crates that depend on its crate; `pub(crate)`, `pub(super)` and `pub(in …)` do not.
pub(crate) const EXPORTED_VISIBILITY: &str = "pub";

/// What a raw identifier is written with before its name.
pub(crate) const RAW_PREFIX: &str = "r#";
