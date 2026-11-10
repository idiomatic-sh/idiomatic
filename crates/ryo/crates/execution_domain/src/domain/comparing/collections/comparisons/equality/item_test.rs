//! The `equality` item's cases.

use super::super::interface::ComparisonsItem as _;
use super::item::EqualityItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::WholeNumber;
use ryo_values::domain::plan_parts::types::{FlagState, Test};

/// A comparison: the test, the left side and the right side.
type Input = (Test, Datum<'static>, Datum<'static>);
type Value = Verdict;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_text_is_equal"),
        word: Word::Succeeds,
        input: || {
            (
                Test::Equal,
                Datum::Text(TextView("a")),
                Datum::Text(TextView("a")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("unlike_values_are_not_equal"),
        word: Word::Succeeds,
        input: || {
            (
                Test::Equal,
                Datum::Number(WholeNumber(1)),
                Datum::Text(TextView("1")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("not_equal_holds_for_unlike_values"),
        word: Word::Succeeds,
        input: || {
            (
                Test::NotEqual,
                Datum::Number(WholeNumber(1)),
                Datum::Number(WholeNumber(2)),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_lowered_flag_is_equal_to_false"),
        word: Word::Succeeds,
        input: || {
            (
                Test::Equal,
                Datum::Flag(FlagState::Lowered),
                Datum::Text(TextView("false")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_raised_flag_is_not_equal_to_false"),
        word: Word::Succeeds,
        input: || {
            (
                Test::Equal,
                Datum::Text(TextView("false")),
                Datum::Flag(FlagState::Raised),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("a_test_of_another_family_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Test::LessThan,
                Datum::Number(WholeNumber(1)),
                Datum::Number(WholeNumber(1)),
            )
        },
        expect: Expect::Error(OperatingError::WrongTest),
    },
];

fn subject((test, left, right): Input) -> Result<Value, Failure> {
    EqualityItem.holds(test, &left, &right)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
