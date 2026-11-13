//! The `spelling` concept's failures.

/// Why the names the files handed over cannot be told apart by their hash.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SpellingError {
    /// Two names with different spellings have the same hash, so the cross-file steps would treat them as one entity.
    #[error("the names `{first}` and `{second}` are in two sources with one hash")]
    EntityInTwoSources {
        /// The spelling met first.
        first: String,
        /// The spelling that has its hash.
        second: String,
    },
}
