//! Regular-expression matching with bounded, reusable compilation storage.

use super::types::{RegexPattern, Patterns};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::Datum;
use regex_automata::meta::Regex;

/// Searches text with a regular expression. Invalid expressions are errors, not empty results.
pub(crate) fn matches(
    left: &Datum<'_>,
    right: &Datum<'_>,
    compiled: &mut Patterns,
) -> Result<Verdict, OperatingError> {
    let (Datum::Text(text), Datum::Text(pattern)) = (left, right) else {
        return Ok(Verdict::Fails);
    };
    if !compiled.0.contains_key(pattern.0) {
        let regex = Regex::new(pattern.0).map_err(|error| OperatingError::InvalidRegex {
            reason: error.to_string(),
        })?;
        if compiled.0.len() >= 128 {
            compiled.0.clear();
        }
        let _previous = compiled.0.insert(RegexPattern(pattern.0.to_owned()), regex);
    }
    Ok(
        if compiled
            .0
            .get(pattern.0)
            .is_some_and(|regex| regex.is_match(text.0))
        {
            Verdict::Holds
        } else {
            Verdict::Fails
        },
    )
}
