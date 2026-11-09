//! The `matching` service's cases: the domain's pattern semantics answer as the matching use case does, whether the parent hands
//! down the use case itself or a trait object of it.

use super::service::HandedMatching;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::matching::types::PatternSemantics;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
use ryo_matching::inbound::use_cases::matching::types::{GlobPattern, MatchOutcome, RegexSource, SearchText};
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;

/// How the parent hands the use case down.
#[derive(Clone, Copy, Debug)]
enum Handed {
    /// The use case itself, through the blanket bridge.
    Itself,
    /// A trait object of it, through `HandedMatching`.
    TraitObject,
}

/// What a row asks: how the use case is handed down, a regex, a text glob and the text both are matched against.
type Input = (Handed, &'static str, &'static str, &'static str);
/// What the regex, the compiled glob and the uncompiled glob answered.
type Value = (MatchOutcome, MatchOutcome, MatchOutcome);
/// The kind of refusal: an invalid regex, whatever its reason.
type Failure = Refusal;

/// Which refusal the port answered with, its reason left out.
#[derive(Debug, PartialEq)]
enum Refusal {
    /// The regex syntax is invalid.
    InvalidRegex,
    /// The glob syntax is invalid.
    BadGlob,
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_use_case_itself_matches_through_the_port"),
        word: Word::Succeeds,
        input: || (Handed::Itself, "^ab+c$", "a?b*", "abbc"),
        expect: Expect::Value((MatchOutcome::Matches, MatchOutcome::Matches, MatchOutcome::Matches)),
    },
    Case {
        name: CaseName("a_trait_object_of_it_matches_through_the_port"),
        word: Word::Succeeds,
        input: || (Handed::TraitObject, "^ab+c$", "x*", "abbc"),
        expect: Expect::Value((MatchOutcome::Matches, MatchOutcome::Differs, MatchOutcome::Differs)),
    },
    Case {
        name: CaseName("an_invalid_regex_is_refused_through_the_port"),
        word: Word::Refuses,
        input: || (Handed::TraitObject, "(", "*", "abbc"),
        expect: Expect::Error(Refusal::InvalidRegex),
    },
];

/// Compiles and matches the row's patterns through the port, handed down as the row says.
fn subject((handed, regex, glob, text): Input) -> Result<Value, Failure> {
    let use_case: &dyn MatchingUseCase = &MatchingUseCaseImpl;
    let trait_object = HandedMatching(use_case);
    let semantics: &dyn PatternSemantics = match handed {
        Handed::Itself => &MatchingUseCaseImpl,
        Handed::TraitObject => &trait_object,
    };
    let compiled = semantics
        .compile_regex(RegexSource(regex))
        .map_err(|error| match error {
            MatchingError::InvalidRegex { .. } => Refusal::InvalidRegex,
            MatchingError::BadGlob { .. } => Refusal::BadGlob,
        })?;
    let globs = semantics.compile_text_globs(&[GlobPattern(glob)]);
    Ok((
        semantics.matches_regex(&compiled, SearchText(text)),
        semantics.matches_text_globs(&globs, SearchText(text)),
        semantics.matches_text_glob(GlobPattern(glob), SearchText(text)),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
