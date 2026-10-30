//! The `patterns` service's cases: the globs a workspace names its member directories with.

use super::service::matches_directory;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::patterns::types::{Directory, Matching, Pattern};

/// What a row asks: whether the directory, the second text, is named by the pattern, the first.
type Input = (&'static str, &'static str);
/// The answer: `yes` or `no`.
type Value = &'static str;
/// The service refuses nothing.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_star_is_a_run_of_characters_inside_one_part"),
        word: Word::Succeeds,
        input: || ("packages/*", "packages/a"),
        expect: Expect::Value("yes"),
    },
    Case {
        name: CaseName("a_star_does_not_cross_a_separator"),
        word: Word::Succeeds,
        input: || ("packages/*", "packages/a/b"),
        expect: Expect::Value("no"),
    },
    Case {
        name: CaseName("two_stars_are_any_number_of_parts"),
        word: Word::Succeeds,
        input: || ("libs/**", "libs/a/b"),
        expect: Expect::Value("yes"),
    },
    Case {
        name: CaseName("a_question_mark_is_one_character"),
        word: Word::Succeeds,
        input: || ("pkg-?", "pkg-1"),
        expect: Expect::Value("yes"),
    },
];

/// Whether the row's pattern names its directory.
fn subject((pattern, directory): Input) -> Value {
    match matches_directory(&Pattern(pattern.into()), Directory(directory)) {
        Matching::Matches => "yes",
        Matching::Differs => "no",
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
