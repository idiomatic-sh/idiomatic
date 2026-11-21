//! Regular-expression matching with bounded, reusable compilation storage, and the glob test: each for one row and for a
//! column of texts.

use super::types::prepared::value::{PreparedMatchers, compile_regex};
use super::types::{PatternMatcher, TextGlobSet};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::patterns::errors::PatternsError;
use crate::domain::table_scratch::types::Patterns;
use crate::domain::table_texts::types::Datum;
use crate::domain::table_texts::types::{RegexPattern, TextView};
use ryo_matching::inbound::use_cases::matching::types::{
    CompiledRegex, GlobPattern, MatchOutcome, SearchText,
};

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
    Ok(found(regex, *text, matcher))
}

/// Whether a compiled expression finds a match in a text: the one statement of what `matches` holds for.
#[inline]
fn found(
    regex: &CompiledRegex,
    text: TextView<'_>,
    matcher: PatternMatcher<'_>,
) -> PredicateOutcome {
    if matcher.0.matches_regex(regex, SearchText(text.0)) == MatchOutcome::Matches {
        PredicateOutcome::Holds
    } else {
        PredicateOutcome::Fails
    }
}

/// The regular-expression test over many texts: for each text, in order, whether the expression finds a match in it.
/// The expression is compiled once, into `compiled`, and every text is searched with it: the same meaning as `matches`,
/// shaped for a caller that holds a column of texts.
///
/// # Errors
///
/// The expression's own error when it is not a valid regular expression.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
/// use ryo_operators::domain::contract::types::PredicateOutcome;
/// use ryo_operators::domain::patterns::service::matches_each;
/// use ryo_operators::domain::patterns::types::PatternMatcher;
/// use ryo_operators::domain::table_scratch::types::Patterns;
/// use ryo_operators::domain::table_texts::types::TextView;
///
/// let mut compiled = Patterns::default();
/// let texts = ["noqa", "todo"].into_iter().map(TextView);
/// let each = matches_each(texts, TextView("^no"), &mut compiled, PatternMatcher(&MatchingUseCaseImpl))
///     .expect("a valid expression");
/// assert_eq!(each.collect::<Vec<_>>(), [PredicateOutcome::Holds, PredicateOutcome::Fails]);
/// ```
pub fn matches_each<'a>(
    texts: impl Iterator<Item = TextView<'a>> + 'a,
    pattern: TextView<'_>,
    compiled: &'a mut Patterns,
    matcher: PatternMatcher<'a>,
) -> Result<impl Iterator<Item = PredicateOutcome> + 'a, PatternsError> {
    let regex = self::compiled(pattern, compiled, matcher)?;
    Ok(texts.map(move |text| found(regex, text, matcher)))
}

/// Whether the text matches any of the globs, given as one text or as several: it holds when one does. The meaning of
/// `glob_any`; `not glob_any` is its negation. A left side that is no text matches nothing.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
/// use ryo_operators::domain::contract::types::PredicateOutcome;
/// use ryo_operators::domain::patterns::service::glob_any;
/// use ryo_operators::domain::patterns::types::PatternMatcher;
/// use ryo_operators::domain::table_texts::types::{Datum, TextView};
///
/// let matcher = PatternMatcher(&MatchingUseCaseImpl);
/// let held = glob_any(&Datum::Text(TextView("test_one")), &Datum::Text(TextView("test_*")), matcher);
/// assert_eq!(held, PredicateOutcome::Holds);
/// ```
#[must_use]
pub fn glob_any(
    text: &Datum<'_>,
    globs: &Datum<'_>,
    matcher: PatternMatcher<'_>,
) -> PredicateOutcome {
    let Datum::Text(text) = text else {
        return PredicateOutcome::Fails;
    };
    match globs {
        Datum::Text(glob) => any_glob(*text, std::iter::once(GlobPattern(glob.0)), matcher),
        Datum::Texts(globs) => any_glob(
            *text,
            globs.iter().map(|glob| GlobPattern(&glob.0)),
            matcher,
        ),
        Datum::Unlocated | Datum::Number(_) | Datum::Flag(_) | Datum::Numbers(_) => {
            PredicateOutcome::Fails
        }
    }
}

/// Whether a text matches one of the globs: the one statement of what `glob_any` holds for.
#[inline]
fn any_glob<'g>(
    text: TextView<'_>,
    mut globs: impl Iterator<Item = GlobPattern<'g>>,
    matcher: PatternMatcher<'_>,
) -> PredicateOutcome {
    let matched = globs
        .any(|glob| matcher.0.matches_text_glob(glob, SearchText(text.0)) == MatchOutcome::Matches);
    if matched {
        PredicateOutcome::Holds
    } else {
        PredicateOutcome::Fails
    }
}

/// The glob test over many texts: for each text, in order, whether it matches any of the globs. The same meaning as
/// `glob_any`, one glob after the other for each text, shaped for a caller that holds a column of texts.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
/// use ryo_operators::domain::contract::types::PredicateOutcome;
/// use ryo_operators::domain::patterns::service::glob_any_each;
/// use ryo_operators::domain::patterns::types::PatternMatcher;
/// use ryo_operators::domain::table_texts::types::TextView;
///
/// let globs = [TextView("test_*"), TextView("*_test")];
/// let texts = ["one_test", "one"].into_iter().map(TextView);
/// let each = glob_any_each(texts, &globs, PatternMatcher(&MatchingUseCaseImpl));
/// assert_eq!(each.collect::<Vec<_>>(), [PredicateOutcome::Holds, PredicateOutcome::Fails]);
/// ```
pub fn glob_any_each<'a>(
    texts: impl Iterator<Item = TextView<'a>> + 'a,
    globs: &'a [TextView<'a>],
    matcher: PatternMatcher<'a>,
) -> impl Iterator<Item = PredicateOutcome> + 'a {
    texts.map(move |text| any_glob(text, globs.iter().map(|glob| GlobPattern(glob.0)), matcher))
}

/// A set of globs compiled once, for a test that asks many texts whether any of the same globs matches: what
/// `glob_any` answers for each text, with the globs read once and not once a text.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
/// use ryo_operators::domain::contract::types::PredicateOutcome;
/// use ryo_operators::domain::patterns::service::{glob_compiled, globs_compiled};
/// use ryo_operators::domain::patterns::types::PatternMatcher;
/// use ryo_operators::domain::table_texts::types::TextView;
///
/// let matcher = PatternMatcher(&MatchingUseCaseImpl);
/// let globs = globs_compiled(&[TextView("a*"), TextView("?d")], matcher);
/// assert_eq!(glob_compiled(TextView("cd"), &globs, matcher), PredicateOutcome::Holds);
/// assert_eq!(glob_compiled(TextView("ef"), &globs, matcher), PredicateOutcome::Fails);
/// ```
#[must_use]
pub fn globs_compiled(globs: &[TextView<'_>], matcher: PatternMatcher<'_>) -> TextGlobSet {
    let patterns: Vec<GlobPattern<'_>> = globs.iter().map(|glob| GlobPattern(glob.0)).collect();
    TextGlobSet(matcher.0.compile_text_globs(&patterns))
}

/// Whether a text matches any glob of a compiled set: `glob_any` for one text, against globs `globs_compiled` read.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
/// use ryo_operators::domain::contract::types::PredicateOutcome;
/// use ryo_operators::domain::patterns::service::{glob_compiled, globs_compiled};
/// use ryo_operators::domain::patterns::types::PatternMatcher;
/// use ryo_operators::domain::table_texts::types::TextView;
///
/// let matcher = PatternMatcher(&MatchingUseCaseImpl);
/// let globs = globs_compiled(&[TextView("kind_?")], matcher);
/// assert_eq!(glob_compiled(TextView("kind_a"), &globs, matcher), PredicateOutcome::Holds);
/// ```
#[must_use]
#[inline]
pub fn glob_compiled(
    text: TextView<'_>,
    globs: &TextGlobSet,
    matcher: PatternMatcher<'_>,
) -> PredicateOutcome {
    if matcher.0.matches_text_globs(&globs.0, SearchText(text.0)) == MatchOutcome::Matches {
        PredicateOutcome::Holds
    } else {
        PredicateOutcome::Fails
    }
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

/// The compiled regex of `pattern` from `compiled`, compiled into it at its first use: one compile per pattern while the
/// cache holds it.
///
/// # Errors
///
/// [`PatternsError::InvalidRegex`] with the matcher's reason when the pattern does not compile.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
/// use ryo_operators::domain::patterns::service::compiled;
/// use ryo_operators::domain::patterns::types::PatternMatcher;
/// use ryo_operators::domain::table_scratch::types::Patterns;
/// use ryo_operators::domain::table_texts::types::TextView;
///
/// let mut kept = Patterns::default();
/// assert!(compiled(TextView("^a"), &mut kept, PatternMatcher(&MatchingUseCaseImpl)).is_ok());
/// assert_eq!(kept.0.len(), 1);
/// ```
pub fn compiled<'a>(
    pattern: TextView<'_>,
    compiled: &'a mut Patterns,
    matcher: PatternMatcher<'_>,
) -> Result<&'a CompiledRegex, PatternsError> {
    compile(pattern, compiled, matcher)?;
    compiled
        .0
        .get(pattern.0)
        .ok_or_else(|| PatternsError::InvalidRegex {
            reason: String::new(),
        })
}
