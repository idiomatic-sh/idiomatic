//! The names of the `parse` use case: how many relations a walk keeps a slot for.

/// Where a language keeps its fixtures, below its directory.
pub const FIXTURES: &str = "tests/fixtures";

/// An unavailable optional reader cannot establish known-empty facts.
pub const UNAVAILABLE_CAPABILITY: &str =
    "The requested optional capability is unavailable from this provider.";

/// The configuration identity of the language that reads nothing: it has no configuration.
pub(super) const NO_CONFIGURATION: &str = "";
