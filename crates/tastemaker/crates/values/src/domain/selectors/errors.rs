//! The `selectors` concept's failures.

/// Why a selector could not be resolved.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SelectorsError {
    /// A glob or regex that does not compile.
    #[error("`{pattern}` is not a pattern: {reason}")]
    Pattern {
        /// The pattern as written.
        pattern: String,
        /// Why it does not compile.
        reason: String,
    },
    /// A `{ variable = … }` that reached placement unresolved.
    #[error("the variable `{name}` was not resolved before placement")]
    Unresolved {
        /// The variable.
        name: String,
    },
}
