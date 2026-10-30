//! The fake of the `matching` port: a pattern matches the paths it is a prefix of, once its `**` is dropped.

use crate::domain::selection::types::{PathMatch, PathPattern};
use crate::domain::violations::types::RepoPath;
use crate::outbound::matching::errors::MatchingError;
use crate::outbound::matching::interface::MatchingPort;

/// What the core's tests stand in for `matching` with.
#[derive(Clone, Copy, Debug, Default)]
pub struct MatchingFake;

impl MatchingPort for MatchingFake {
    fn matches(
        &self,
        patterns: &[PathPattern],
        path: &RepoPath,
    ) -> Result<PathMatch, MatchingError> {
        if let Some(bad) = patterns.iter().find(|pattern| pattern.0.contains('[')) {
            return Err(MatchingError::Pattern {
                pattern: bad.0.to_string(),
                reason: "the fake reads no class".to_owned(),
            });
        }
        let matched = patterns.is_empty()
            || patterns
                .iter()
                .any(|pattern| match pattern.0.strip_suffix("**") {
                    Some(prefix) => path.0.starts_with(prefix),
                    None => !pattern.0.contains('*') && pattern.0 == path.0,
                });
        Ok(if matched {
            PathMatch::Matched
        } else {
            PathMatch::Unmatched
        })
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
