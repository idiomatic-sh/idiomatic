//! What the `matching` port asks of the outside, in the core's own vocabulary.

use super::errors::MatchingError;
use asymptote_values::domain::selection::types::{PathMatch, PathPattern};
use asymptote_values::domain::violations::types::RepoPath;

/// What every `matching` adapter implements.
pub trait MatchingPort: std::fmt::Debug + Send + Sync {
    /// Whether `path` matches any of `patterns`: `**` crosses directories, `*` does not; no pattern matches every
    /// path.
    ///
    /// # Errors
    ///
    /// `Pattern` when one of them is no glob.
    fn matches(
        &self,
        patterns: &[PathPattern],
        path: &RepoPath,
    ) -> Result<PathMatch, MatchingError>;
}
