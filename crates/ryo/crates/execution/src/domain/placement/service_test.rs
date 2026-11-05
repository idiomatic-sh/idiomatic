//! The `placement` service's cases, on values alone.

use super::service::{decide, governs, place};
use super::types::{Governing, Mask, PlacedScopes, RuleScope};
use ryo_language::domain::files::types::FilePath;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase, use_case::MatchingUseCaseImpl,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::files::types::Hits;
use ryo_values::domain::runs::types::RuleNumber;

/// What a row of the cases does: the scopes of the rules, and the path placed.
type Input = (
    &'static [(&'static [&'static str], &'static [&'static str])],
    &'static str,
);
/// Which of the first three rules govern the path, as a number written in binary (rule zero is the lowest bit), and above the eighth
/// bit how many globs were made ready.
type Value = usize;
type Failure = MatchingError;

/// A rule whose only glob is not one.
const BAD_GLOB: &[(&[&str], &[&str])] = &[(&["["], &[])];

/// Rule zero governs everything; rule one only Rust files; rule two everything but tests.
/// Two rules write one glob and a third excludes it: it is made ready once.
const SHARED: &[(&[&str], &[&str])] = &[
    (&["**/*.rs"], &[]),
    (&["**/*.rs"], &[]),
    (&["**/*.md"], &["**/*.rs"]),
];

const SCOPES: &[(&[&str], &[&str])] = &[
    (&[], &[]),
    (&["**/*.rs"], &[]),
    (&[], &["**/tests/**", "**/*_test.rs"]),
];

fn subject((scopes, path): Input) -> Result<Value, Failure> {
    let scopes: Vec<RuleScope> = scopes
        .iter()
        .map(|(include, exclude)| RuleScope {
            include: include.iter().map(|glob| Glob((*glob).into())).collect(),
            exclude: exclude.iter().map(|glob| Glob((*glob).into())).collect(),
        })
        .collect();
    let placement = decide(&scopes);
    let globs = placement.globs.clone();
    let placed = PlacedScopes {
        matcher: compiled_globs(&globs)?,
        placement,
    };
    let (mut mask, mut hits) = (Mask::default(), Hits::default());
    place(&placed, &FilePath(path.into()), &mut mask, &mut hits);
    let bits = (0..3)
        .filter(|rule| governs(&mask, RuleNumber(*rule)) == Governing::Governs)
        .map(|rule| 1 << rule)
        .sum::<usize>();
    Ok(bits | (globs.len() << 8))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rust_source_file_is_governed_by_all_three"),
        word: Word::Succeeds,
        input: || (SCOPES, "src/lib.rs"),
        expect: Expect::Value(0b111 | (3 << 8_i32)),
    },
    Case {
        name: CaseName("a_test_file_is_not_governed_by_the_rule_that_leaves_tests_out"),
        word: Word::Succeeds,
        input: || (SCOPES, "src/lib_test.rs"),
        expect: Expect::Value(0b011 | (3 << 8_i32)),
    },
    Case {
        name: CaseName("a_text_file_is_governed_by_the_rules_that_include_nothing"),
        word: Word::Succeeds,
        input: || (SCOPES, "notes.txt"),
        expect: Expect::Value(0b101 | (3 << 8_i32)),
    },
    Case {
        name: CaseName("a_glob_several_rules_write_is_made_ready_once_and_is_each_ones"),
        word: Word::Succeeds,
        input: || (SHARED, "a.rs"),
        expect: Expect::Value(0b011 | (2 << 8_i32)),
    },
    Case {
        name: CaseName("a_glob_that_is_not_one_is_refused"),
        word: Word::Refuses,
        input: || (BAD_GLOB, "a.rs"),
        expect: Expect::Refusal(|error| selects(matches!(error, MatchingError::BadGlob { .. }))),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

// Compile the actual external semantic input; the subject remains pure placement.
fn compiled_globs(
    globs: &[Glob],
) -> Result<ryo_values::domain::files::types::GlobsMatcher, MatchingError> {
    let compiled = MatchingUseCaseImpl::new()
        .compile_globs(&globs.iter().map(|glob| glob.0.as_ref()).collect::<Vec<_>>())?;
    Ok(ryo_values::domain::files::types::GlobsMatcher {
        matches: std::sync::Arc::new(
            move |path: ryo_values::domain::files::types::MatchCandidate<'_>, hits: &mut Hits| {
                MatchingUseCaseImpl::new().match_globs(
                    &compiled,
                    std::path::Path::new(path.0),
                    &mut hits.0,
                );
            },
        ),
    })
}
