//! The `ordering` item's cases.

use super::super::interface::ComparisonsItem as _;
use super::item::OrderingItem;
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
        name: CaseName("greater_than_holds"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GreaterThan,
                Datum::Number(WholeNumber(5)),
                Datum::Number(WholeNumber(4)),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_number_and_a_text_are_never_ordered"),
        word: Word::Succeeds,
        input: || {
            (
                Test::LessThan,
                Datum::Number(WholeNumber(5)),
                Datum::Text(TextView("9")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("texts_are_ordered_byte_by_byte"),
        word: Word::Succeeds,
        input: || {
            (
                Test::LessThan,
                Datum::Text(TextView("crates/a")),
                Datum::Text(TextView("crates/b")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_text_is_not_less_than_itself"),
        word: Word::Succeeds,
        input: || {
            (
                Test::LessThan,
                Datum::Text(TextView("same")),
                Datum::Text(TextView("same")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("a_text_is_at_least_itself"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GreaterThanOrEqual,
                Datum::Text(TextView("same")),
                Datum::Text(TextView("same")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_longer_text_is_greater_than_its_prefix"),
        word: Word::Succeeds,
        input: || {
            (
                Test::GreaterThan,
                Datum::Text(TextView("ab")),
                Datum::Text(TextView("a")),
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
                Datum::Number(WholeNumber(1)),
                Datum::Number(WholeNumber(1)),
            )
        },
        expect: Expect::Error(OperatingError::WrongTest),
    },
];

fn subject((test, left, right): Input) -> Result<Value, Failure> {
    OrderingItem.holds(test, &left, &right)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
