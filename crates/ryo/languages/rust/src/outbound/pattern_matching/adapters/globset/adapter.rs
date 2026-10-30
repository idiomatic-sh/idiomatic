//! The matching port, implemented with `globset`: a `*` stays within a segment.

use super::types::Matcher;
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::Matching;
use crate::domain::targets::types::Pattern;
use crate::outbound::pattern_matching::errors::PatternMatchingError;
use crate::outbound::pattern_matching::interface::PatternMatchingPort;

/// The `globset` matching adapter.
#[derive(Debug)]
pub(crate) struct PatternMatchingGlobsetAdapter;

impl PatternMatchingPort for PatternMatchingGlobsetAdapter {
    fn matches(
        &self,
        pattern: &Pattern,
        text: &EntryName,
    ) -> Result<Matching, PatternMatchingError> {
        Matcher::try_from(&*pattern.0).map(|matcher| {
            if matcher.compiled.is_match(&*text.0) {
                Matching::Matches
            } else {
                Matching::Differs
            }
        })
    }
}
