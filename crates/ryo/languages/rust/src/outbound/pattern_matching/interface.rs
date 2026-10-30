//! What the `pattern_matching` port asks of the outside: whether a path matches a pattern a manifest writes.

use super::errors::PatternMatchingError;
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::Matching;
use crate::domain::targets::types::Pattern;

/// What every `pattern_matching` adapter implements.
pub(crate) trait PatternMatchingPort: Send + Sync {
    /// Whether `text`, a path, matches `pattern`, where `*` and `?` and `[..]` stand for part of one segment of the path and
    /// never for a `/`.
    ///
    /// # Errors
    ///
    /// `Invalid` when the pattern is not one.
    fn matches(
        &self,
        pattern: &Pattern,
        text: &EntryName,
    ) -> Result<Matching, PatternMatchingError>;
}
