//! The shape of a pattern, which the language does not control, and its checked conversion into a matcher: a `*` stays within a
//! segment, and a pattern that is not one is refused.

use crate::domain::targets::types::Pattern;
use crate::domain::workspaces::types::Reason;
use globset::GlobBuilder;

use super::types::Matcher;
use crate::outbound::pattern_matching::errors::PatternMatchingError;

/// A pattern as a manifest writes it.
#[derive(Clone, Debug)]
pub(super) struct PatternSchema(String);

impl TryFrom<PatternSchema> for Matcher {
    type Error = PatternMatchingError;

    fn try_from(schema: PatternSchema) -> Result<Self, Self::Error> {
        let glob = GlobBuilder::new(&schema.0)
            .literal_separator(true)
            .build()
            .map_err(|error| PatternMatchingError::Invalid {
                pattern: Pattern(schema.0.as_str().into()),
                reason: Reason(error.kind().to_string().into()),
            })?;
        Ok(Self {
            compiled: glob.compile_matcher(),
        })
    }
}

impl TryFrom<&str> for Matcher {
    type Error = PatternMatchingError;

    fn try_from(pattern: &str) -> Result<Self, Self::Error> {
        Self::try_from(PatternSchema(pattern.to_owned()))
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
