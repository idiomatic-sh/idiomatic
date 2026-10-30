//! The matching port, implemented with globset: `*` stops at a `/`, as git's and Tastemaker's globs do.

use globset::{GlobBuilder, GlobSetBuilder};

use super::constants::JOIN;
use crate::domain::selection::types::{PathMatch, PathPattern};
use crate::domain::violations::types::RepoPath;
use crate::outbound::matching::errors::MatchingError;
use crate::outbound::matching::interface::MatchingPort;

/// The globset matching adapter.
#[derive(Debug)]
pub(crate) struct MatchingGlobsetAdapter;

impl MatchingPort for MatchingGlobsetAdapter {
    fn matches(
        &self,
        patterns: &[PathPattern],
        path: &RepoPath,
    ) -> Result<PathMatch, MatchingError> {
        if patterns.is_empty() {
            return Ok(PathMatch::Matched);
        }
        let globs = patterns
            .iter()
            .map(|pattern| {
                GlobBuilder::new(&pattern.0)
                    .literal_separator(true)
                    .build()
                    .map_err(|error| MatchingError::Pattern {
                        pattern: pattern.0.to_string(),
                        reason: error.kind().to_string(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut builder = GlobSetBuilder::new();
        let set = globs
            .into_iter()
            .fold(&mut builder, |builder, glob| builder.add(glob))
            .build()
            .map_err(|error| MatchingError::Pattern {
                pattern: patterns
                    .iter()
                    .map(|pattern| &*pattern.0)
                    .collect::<Vec<_>>()
                    .join(JOIN),
                reason: error.to_string(),
            })?;
        Ok(if set.is_match(&*path.0) {
            PathMatch::Matched
        } else {
            PathMatch::Unmatched
        })
    }
}
