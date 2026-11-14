//! The `membership` item's cases.

use std::sync::LazyLock;

use super::super::interface::ComparisonsItem as _;
use super::item::MembershipItem;
use crate::domain::comparing::errors::ComparingError;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plan_parts::types::WholeNumber;

/// The members the cases use.
static ONLY_A: LazyLock<[LiteralText; 1]> = LazyLock::new(|| [LiteralText("a".into())]);

/// No numbers at all.
const NO_NUMBERS: &[WholeNumber] = &[];

/// A comparison: the test, the left side and the right side.
type Input = (Test, Datum<'static>, Datum<'static>);
type Value = PredicateOutcome;
type Failure = ComparingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_member_is_in"),
        word: Word::Succeeds,
        input: || (Test::In, Datum::Text(TextView("a")), Datum::Texts(&*ONLY_A)),
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("not_in_holds_for_a_stranger"),
        word: Word::Succeeds,
        input: || {
            (
                Test::NotIn,
                Datum::Number(WholeNumber(2)),
                Datum::Numbers(&[WholeNumber(1)]),
            )
        },
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("a_test_of_another_family_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Test::Equal,
                Datum::Number(WholeNumber(1)),
                Datum::Numbers(NO_NUMBERS),
            )
        },
        expect: Expect::Error(ComparingError::WrongTest),
    },
];

fn subject((test, left, right): Input) -> Result<Value, Failure> {
    MembershipItem.holds(test, &left, &right)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
