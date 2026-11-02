//! Regular-expression matching with bounded, reusable compilation storage.

use super::types::prepared::value::PreparedMatchers;
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
    matches_prepared(left, right, None, compiled)
}

/// Use authored request metadata only when its owning comparison supplies a literal pattern.
/// Type mismatch must return before any lazy pattern slot is initialized.
pub(crate) fn matches_prepared(
    left: &Datum<'_>,
    right: &Datum<'_>,
    prepared: Option<&PreparedMatchers>,
    compiled: &mut Patterns,
) -> Result<Verdict, OperatingError> {
    let (Datum::Text(text), Datum::Text(pattern)) = (left, right) else {
        return Ok(Verdict::Fails);
    };
    Ok(if regex(*pattern, prepared, compiled)?.is_match(text.0) {
        Verdict::Holds
    } else {
        Verdict::Fails
    })
}

/// Borrow a prepared literal matcher, or keep dynamic/undeclared patterns in their invocation cache.
fn regex<'a>(
    pattern: TextView<'_>,
    prepared: Option<&'a PreparedMatchers>,
    compiled: &'a mut Patterns,
) -> Result<&'a Regex, OperatingError> {
    if let Some(ready) = prepared.and_then(|prepared| prepared.regex(pattern.0)) {
        return ready;
    }
    compile(pattern, compiled)?;
    compiled
        .0
        .get(pattern.0)
        .ok_or(OperatingError::WrongBinding)
}

/// Compile an invocation-owned pattern in the bounded operator cache.
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
    prepared: Option<&PreparedMatchers>,
    compiled: &mut Patterns,
) -> Result<LiteralText, OperatingError> {
    let regex = regex(pattern, prepared, compiled)?;
    let mut output = String::with_capacity(text.0.len());
    let mut end = 0;
    for found in regex.find_iter(text.0) {
        output.push_str(
            text.0
                .get(end..found.start())
                .ok_or(OperatingError::ShapeMismatch)?,
        );
        output.push_str(replacement.0);
        end = found.end();
    }
    output.push_str(text.0.get(end..).ok_or(OperatingError::ShapeMismatch)?);
    Ok(LiteralText(output.into()))
}
