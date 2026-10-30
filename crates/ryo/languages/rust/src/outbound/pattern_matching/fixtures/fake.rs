//! The fake of the `pattern_matching` port: what the language's tests stand in for it with.

use super::super::errors::PatternMatchingError;
use super::super::interface::PatternMatchingPort;
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::Matching;
use crate::domain::targets::types::Pattern;
use crate::domain::workspaces::types::Reason;

/// What the language's tests stand in for `pattern_matching` with: a path matches a pattern that is the path, or the path of a directory
/// with a `*` in place of the name of one thing in it.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PatternMatchingFake;

impl PatternMatchingPort for PatternMatchingFake {
    fn matches(
        &self,
        pattern: &Pattern,
        text: &EntryName,
    ) -> Result<Matching, PatternMatchingError> {
        let (pattern, text): (&str, &str) = (&pattern.0, &text.0);
        if pattern.contains('[') {
            return Err(PatternMatchingError::Invalid {
                pattern: Pattern(pattern.into()),
                reason: Reason("the fake knows no brackets".into()),
            });
        }
        let matched = match pattern.strip_suffix("/*") {
            Some(directory) => text
                .strip_prefix(directory)
                .and_then(|rest| rest.strip_prefix('/'))
                .is_some_and(|name| !name.is_empty() && !name.contains('/')),
            None => pattern == text,
        };
        Ok(if matched {
            Matching::Matches
        } else {
            Matching::Differs
        })
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
