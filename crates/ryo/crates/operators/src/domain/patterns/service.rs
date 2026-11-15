//! Regular-expression matching with bounded, reusable compilation storage.

use super::types::PatternMatcher;
use super::types::prepared::value::{PreparedMatchers, compile_regex};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::patterns::errors::PatternsError;
use crate::domain::table_scratch::types::Patterns;
use crate::domain::table_texts::types::Datum;
use crate::domain::table_texts::types::{RegexPattern, TextView};
use ryo_matching::inbound::use_cases::matching::types::{CompiledRegex, MatchOutcome, SearchText};

/// Searches text with a regular expression. Invalid expressions are errors, not empty results.
///
/// # Errors
///
/// The expression's own error when it is not a valid regular expression.
pub fn matches(
    left: &Datum<'_>,
    right: &Datum<'_>,
    compiled: &mut Patterns,
    matcher: PatternMatcher<'_>,
) -> Result<PredicateOutcome, PatternsError> {
    matches_prepared(left, right, None, compiled, matcher)
}

/// Use authored request metadata only when its owning comparison supplies a literal pattern.
/// Type mismatch must return before any lazy pattern slot is initialized.
pub(crate) fn matches_prepared(
    left: &Datum<'_>,
    right: &Datum<'_>,
    prepared: Option<&PreparedMatchers>,
    compiled: &mut Patterns,
    matcher: PatternMatcher<'_>,
) -> Result<PredicateOutcome, PatternsError> {
    let (Datum::Text(text), Datum::Text(pattern)) = (left, right) else {
        return Ok(PredicateOutcome::Fails);
    };
    // A pattern the cache does not hold right after compiling it into the cache matches nothing.
    let Some(regex) = regex(*pattern, prepared, compiled, matcher)? else {
        return Ok(PredicateOutcome::Fails);
    };
    Ok(
        if matcher.0.matches_regex(regex, SearchText(text.0)) == MatchOutcome::Matches {
            PredicateOutcome::Holds
        } else {
            PredicateOutcome::Fails
        },
    )
}

/// Borrow a prepared literal matcher, or keep dynamic/undeclared patterns in their invocation cache.
fn regex<'a>(
    pattern: TextView<'_>,
    prepared: Option<&'a PreparedMatchers>,
    compiled: &'a mut Patterns,
    matcher: PatternMatcher<'_>,
) -> Result<Option<&'a CompiledRegex>, PatternsError> {
    if let Some(ready) = prepared.and_then(|prepared| prepared.regex(pattern, matcher)) {
        return ready.map(Some);
    }
    compile(pattern, compiled, matcher)?;
    Ok(compiled.0.get(pattern.0))
}

/// Compile an invocation-owned pattern in the bounded operator cache.
fn compile(
    pattern: TextView<'_>,
    compiled: &mut Patterns,
    matcher: PatternMatcher<'_>,
) -> Result<(), PatternsError> {
    if !compiled.0.contains_key(pattern.0) {
        let regex = compile_regex(pattern, matcher)?;
        if compiled.0.len() >= 128 {
            compiled.0.clear();
        }
        let _previous = compiled.0.insert(RegexPattern(pattern.0.to_owned()), regex);
    }
    Ok(())
}
