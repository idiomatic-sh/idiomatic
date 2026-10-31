//! The names of the `parse` use case: how many relations a walk keeps a slot for.

/// How many relations the catalog has.
pub(super) const COUNT: usize = crate::domain::relations::service::ALL.len();

/// How many numbers a set of relations takes, a bit for each.
pub(super) const WORDS: usize = COUNT.div_ceil(64);

/// Where a language keeps its fixtures, below its directory.
pub const FIXTURES: &str = "tests/fixtures";

/// An unavailable optional reader cannot establish known-empty facts.
pub const UNAVAILABLE_CAPABILITY: &str =
    "The requested optional capability is unavailable from this provider.";
