//! Numeric comparison is distinct from ordinary equality and finite textual-float comparison.

use super::super::interface::ComparisonsItem as _;
use super::item::NumericItem;
use crate::domain::comparing::collections::comparisons::float_text::item::FloatTextItem;
use crate::domain::comparing::errors::ComparingError;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{FlagState, Test, WholeNumber};

/// The family a row asks.
enum Family {
    /// The exact numeric family under test.
    Numeric,
    /// The finite-only float-text family, which stays separate.
    FloatText,
}
/// A comparison: the family, the test, the left side and the right side.
type Input = (Family, Test, Datum<'static>, Datum<'static>);
type Value = PredicateOutcome;
type Failure = ComparingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("text_of_a_large_integer_holds_against_the_same_whole_number"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::NumericGreaterThanOrEqual,
                Datum::Text(TextView("9007199254740993")),
                Datum::Number(WholeNumber(9_007_199_254_740_993)),
            )
        },
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("a_whole_number_below_the_text_integer_fails"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::NumericGreaterThanOrEqual,
                Datum::Number(WholeNumber(9_007_199_254_740_992)),
                Datum::Text(TextView("9007199254740993")),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("infinity_is_at_least_a_finite_float"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::NumericGreaterThanOrEqual,
                Datum::Text(TextView("inf")),
                Datum::Text(TextView("1.0")),
            )
        },
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("nan_is_not_at_least_nan"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::NumericGreaterThanOrEqual,
                Datum::Text(TextView("NaN")),
                Datum::Text(TextView("NaN")),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("a_source_language_spelling_fails"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::NumericGreaterThanOrEqual,
                Datum::Text(TextView("0xff")),
                Datum::Number(WholeNumber(0)),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("a_flag_fails"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::NumericGreaterThanOrEqual,
                Datum::Flag(FlagState::Raised),
                Datum::Number(WholeNumber(1)),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("an_unlocated_cell_fails"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::NumericGreaterThanOrEqual,
                Datum::Unlocated,
                Datum::Number(WholeNumber(0)),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("a_list_of_numbers_fails"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::NumericGreaterThanOrEqual,
                Datum::Numbers(&[]),
                Datum::Number(WholeNumber(0)),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName(
            "the_finite_float_text_family_refuses_infinity_that_the_numeric_family_accepts",
        ),
        word: Word::Succeeds,
        input: || {
            (
                Family::FloatText,
                Test::FloatTextGreaterThanOrEqual,
                Datum::Text(TextView("inf")),
                Datum::Text(TextView("1.0")),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("the_numeric_family_accepts_infinity_against_a_finite_float"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::NumericGreaterThanOrEqual,
                Datum::Text(TextView("inf")),
                Datum::Text(TextView("1.0")),
            )
        },
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("a_whole_number_precedes_its_text_successor"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::IntegerSuccessor,
                Datum::Number(WholeNumber(-1)),
                Datum::Text(TextView("0")),
            )
        },
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("text_integers_beyond_float_precision_have_a_successor"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::IntegerSuccessor,
                Datum::Text(TextView("9007199254740992")),
                Datum::Text(TextView("9007199254740993")),
            )
        },
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("a_float_left_has_no_successor"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::IntegerSuccessor,
                Datum::Text(TextView("1.0")),
                Datum::Number(WholeNumber(2)),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("a_float_right_is_no_successor"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::IntegerSuccessor,
                Datum::Text(TextView("1")),
                Datum::Text(TextView("2.0")),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("a_source_language_spelling_has_no_successor"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::IntegerSuccessor,
                Datum::Text(TextView("0xff")),
                Datum::Number(WholeNumber(256)),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("a_flag_has_no_successor"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::IntegerSuccessor,
                Datum::Flag(FlagState::Raised),
                Datum::Number(WholeNumber(2)),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("nan_is_no_successor"),
        word: Word::Succeeds,
        input: || {
            (
                Family::Numeric,
                Test::IntegerSuccessor,
                Datum::Text(TextView("1")),
                Datum::Text(TextView("NaN")),
            )
        },
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("a_test_of_another_family_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Family::Numeric,
                Test::Equal,
                Datum::Number(WholeNumber(1)),
                Datum::Number(WholeNumber(1)),
            )
        },
        expect: Expect::Error(ComparingError::WrongTest),
    },
];

fn subject((family, test, left, right): Input) -> Result<Value, Failure> {
    match family {
        Family::Numeric => NumericItem.holds(test, &left, &right),
        Family::FloatText => FloatTextItem.holds(test, &left, &right),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
