//! The `float_text` item's cases.

use super::super::interface::ComparisonsItem as _;
use super::item::FloatTextItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plan_parts::types::WholeNumber;

/// A comparison: the test, the left side and the right side.
type Input = (Test, Datum<'static>, Datum<'static>);
type Value = Verdict;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("numbers_written_as_text_compare_as_numbers"),
        word: Word::Succeeds,
        input: || {
            (
                Test::FloatTextGreaterThanOrEqual,
                Datum::Text(TextView("10.5")),
                Datum::Text(TextView("9")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("text_that_is_not_a_number_never_holds"),
        word: Word::Succeeds,
        input: || {
            (
                Test::FloatTextGreaterThanOrEqual,
                Datum::Text(TextView("ten")),
                Datum::Text(TextView("9")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("a_test_of_another_family_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Test::Equal,
                Datum::Number(WholeNumber(1)),
                Datum::Number(WholeNumber(1)),
            )
        },
        expect: Expect::Error(OperatingError::WrongTest),
    },
];

fn subject((test, left, right): Input) -> Result<Value, Failure> {
    FloatTextItem.holds(test, &left, &right)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
