//! The `glob` item's cases.

use std::sync::LazyLock;

use super::super::interface::ComparisonsItem as _;
use super::item::GlobItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Test;

/// The globs the cases use.
static ANY_RUST: LazyLock<[LiteralText; 1]> = LazyLock::new(|| [LiteralText("**/*.rs".into())]);

/// No globs at all.
const NO_GLOBS: &[LiteralText] = &[];

/// A comparison: the test, the left side and the right side.
type Input = (Test, Datum<'static>, Datum<'static>);
type Value = Verdict;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_star_crosses_directories_as_today"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("src/a/b.rs")),
                Datum::Text(TextView("src/*.rs")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_double_star_slash_between_names_may_be_no_directory"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("src/lib.rs")),
                Datum::Text(TextView("src/**/lib.rs")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_double_star_slash_keeps_the_names_whole"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("src/xlib.rs")),
                Datum::Text(TextView("src/**/lib.rs")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("stars_around_a_word_find_it_anywhere"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("// first TODO later")),
                Datum::Text(TextView("{*TODO*,*FIXME*}")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_class_of_digits_takes_a_digit"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("131072i32")),
                Datum::Text(TextView("[0-9][0-9][0-9][0-9][0-9][0-9]*")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_class_of_digits_does_not_take_a_letter"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("13107x")),
                Datum::Text(TextView("[0-9][0-9][0-9][0-9][0-9][0-9]*")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("a_negated_class_takes_what_it_does_not_list"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("ab")),
                Datum::Text(TextView("[!0-9]b")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("braces_are_alternatives"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("if-expr")),
                Datum::Text(TextView("{if-expr,match-expr}")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_double_star_slash_may_be_no_directory"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("adapter.rs")),
                Datum::Text(TextView("**/adapter.rs")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_name_outside_the_braces_is_no_match"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("loop-expr")),
                Datum::Text(TextView("{if-expr,match-expr}")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("a_brace_inside_a_class_is_a_character_not_an_alternative"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("a @{case} b")),
                Datum::Text(TextView("a @[{]case[}] b")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("groups_may_nest_and_follow_one_another"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("a_x_d")),
                Datum::Text(TextView("{{a,b}_{x,y},c}_{d,e}")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_backslash_makes_the_next_character_itself"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("C:\\cache")),
                Datum::Text(TextView("?:\\\\*")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_double_star_crosses_directories"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GlobAny,
                Datum::Text(TextView("src/a/b.rs")),
                Datum::Texts(&*ANY_RUST),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("an_empty_collection_keeps_the_row_for_not_glob_any"),
        word: Word::Succeeds,
        input: || {
            (
                Test::NotGlobAny,
                Datum::Text(TextView("a.rs")),
                Datum::Texts(NO_GLOBS),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_test_of_another_family_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Test::Equal,
                Datum::Text(TextView("a")),
                Datum::Texts(NO_GLOBS),
            )
        },
        expect: Expect::Error(OperatingError::WrongTest),
    },
];

fn subject((test, left, right): Input) -> Result<Value, Failure> {
    GlobItem(crate::domain::matching::types::Matcher(
        &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
    ))
    .holds(test, &left, &right)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
