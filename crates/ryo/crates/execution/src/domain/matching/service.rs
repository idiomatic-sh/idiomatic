//! Regular-expression matching with bounded, reusable compilation storage.

use super::types::Matcher;
use super::types::prepared::value::{PreparedMatchers, compile_regex};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_scratch::types::Patterns;
use crate::domain::table_texts::types::Datum;
use crate::domain::table_texts::types::{RegexPattern, TextView};
use ryo_matching::inbound::use_cases::matching::{
    compiled::CompiledRegex,
    types::{MatchOutcome, SearchText},
};

/// Searches text with a regular expression. Invalid expressions are errors, not empty results.
pub(crate) fn matches(
    left: &Datum<'_>,
    right: &Datum<'_>,
    compiled: &mut Patterns,
    matcher: Matcher<'_>,
) -> Result<Verdict, OperatingError> {
    matches_prepared(left, right, None, compiled, matcher)
}

/// Use authored request metadata only when its owning comparison supplies a literal pattern.
/// Type mismatch must return before any lazy pattern slot is initialized.
pub(crate) fn matches_prepared(
    left: &Datum<'_>,
    right: &Datum<'_>,
    prepared: Option<&PreparedMatchers>,
    compiled: &mut Patterns,
    matcher: Matcher<'_>,
) -> Result<Verdict, OperatingError> {
    let (Datum::Text(text), Datum::Text(pattern)) = (left, right) else {
        return Ok(Verdict::Fails);
    };
    Ok(
        if matcher
            .0
            .matches_regex(regex(*pattern, prepared, compiled, matcher)?, SearchText(text.0))
            == MatchOutcome::Matches
        {
            Verdict::Holds
        } else {
            Verdict::Fails
        },
    )
}

/// Borrow a prepared literal matcher, or keep dynamic/undeclared patterns in their invocation cache.
fn regex<'a>(
    pattern: TextView<'_>,
    prepared: Option<&'a PreparedMatchers>,
    compiled: &'a mut Patterns,
    matcher: Matcher<'_>,
) -> Result<&'a CompiledRegex, OperatingError> {
    if let Some(ready) = prepared.and_then(|prepared| prepared.regex(pattern, matcher)) {
        return ready;
    }
    compile(pattern, compiled, matcher)?;
    compiled
        .0
        .get(pattern.0)
        .ok_or(OperatingError::WrongBinding)
}

/// Compile an invocation-owned pattern in the bounded operator cache.
fn compile(
    pattern: TextView<'_>,
    compiled: &mut Patterns,
    matcher: Matcher<'_>,
) -> Result<(), OperatingError> {
    if !compiled.0.contains_key(pattern.0) {
        let regex = compile_regex(pattern, matcher)?;
        if compiled.0.len() >= 128 {
            compiled.0.clear();
        }
        let _previous = compiled.0.insert(RegexPattern(pattern.0.to_owned()), regex);
    }
    Ok(())
}
