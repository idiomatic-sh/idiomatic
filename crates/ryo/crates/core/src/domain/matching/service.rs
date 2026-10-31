//! Regular-expression matching with bounded, reusable compilation storage.

use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_scratch::types::Patterns;
use crate::domain::table_texts::types::Datum;
use crate::domain::table_texts::types::{RegexPattern, TextView};
use regex_automata::meta::Regex;
use ryo_values::domain::plan_parts::types::LiteralText;

/// Searches text with a regular expression. Invalid expressions are errors, not empty results.
pub(crate) fn matches(
    left: &Datum<'_>,
    right: &Datum<'_>,
    compiled: &mut Patterns,
) -> Result<Verdict, OperatingError> {
    let (Datum::Text(text), Datum::Text(pattern)) = (left, right) else {
        return Ok(Verdict::Fails);
    };
    compile(*pattern, compiled)?;
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

/// Compile a constant pattern once in the bounded operator cache.
fn compile(pattern: TextView<'_>, compiled: &mut Patterns) -> Result<(), OperatingError> {
    if !compiled.0.contains_key(pattern.0) {
        let regex = Regex::new(pattern.0).map_err(|error| OperatingError::InvalidRegex {
            reason: error.to_string(),
        })?;
        if compiled.0.len() >= 128 {
            compiled.0.clear();
        }
        let _previous = compiled.0.insert(RegexPattern(pattern.0.to_owned()), regex);
    }
    Ok(())
}

/// Replace every regex match with literal text, preserving the unmatched source bytes.
pub(crate) fn replace(
    text: TextView<'_>,
    pattern: TextView<'_>,
    replacement: TextView<'_>,
    compiled: &mut Patterns,
) -> Result<LiteralText, OperatingError> {
    compile(pattern, compiled)?;
    let regex = compiled
        .0
        .get(pattern.0)
        .ok_or(OperatingError::WrongBinding)?;
    let mut output = String::with_capacity(text.0.len());
    let mut end = 0;
    for found in regex.find_iter(text.0) {
        output.push_str(&text.0[end..found.start()]);
        output.push_str(replacement.0);
        end = found.end();
    }
    output.push_str(&text.0[end..]);
    Ok(LiteralText(output.into()))
}
