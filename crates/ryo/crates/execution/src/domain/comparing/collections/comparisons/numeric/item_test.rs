//! Numeric comparison is distinct from ordinary equality and finite textual-float comparison.

use super::item::NumericItem;
use crate::domain::comparing::collections::comparisons::{
    float_text::item::FloatTextItem, interface::ComparisonsItem as _,
};
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::domain::plan_parts::types::{FlagState, Test, WholeNumber};

#[test]
fn numeric_gte_accepts_text_or_whole_numbers_but_not_flags_or_lists() {
    let gte = Test::NumericGreaterThanOrEqual;
    for (left, right, expected) in [
        (
            Datum::Text(TextView("9007199254740993")),
            Datum::Number(WholeNumber(9_007_199_254_740_993)),
            Verdict::Holds,
        ),
        (
            Datum::Number(WholeNumber(9_007_199_254_740_992)),
            Datum::Text(TextView("9007199254740993")),
            Verdict::Fails,
        ),
        (
            Datum::Text(TextView("inf")),
            Datum::Text(TextView("1.0")),
            Verdict::Holds,
        ),
        (
            Datum::Text(TextView("NaN")),
            Datum::Text(TextView("NaN")),
            Verdict::Fails,
        ),
        (
            Datum::Text(TextView("0xff")),
            Datum::Number(WholeNumber(0)),
            Verdict::Fails,
        ),
        (
            Datum::Flag(FlagState::Raised),
            Datum::Number(WholeNumber(1)),
            Verdict::Fails,
        ),
        (
            Datum::Unlocated,
            Datum::Number(WholeNumber(0)),
            Verdict::Fails,
        ),
        (
            Datum::Numbers(&[]),
            Datum::Number(WholeNumber(0)),
            Verdict::Fails,
        ),
    ] {
        assert_eq!(
            NumericItem
                .holds(gte, &left, &right)
                .expect("numeric family"),
            expected
        );
    }
    assert!(
        NumericItem
            .holds(
                Test::Equal,
                &Datum::Number(WholeNumber(1)),
                &Datum::Number(WholeNumber(1))
            )
            .is_err()
    );
}

#[test]
fn finite_only_float_text_contract_remains_separate() {
    let left = Datum::Text(TextView("inf"));
    let right = Datum::Text(TextView("1.0"));
    assert_eq!(
        FloatTextItem
            .holds(Test::FloatTextGreaterThanOrEqual, &left, &right)
            .expect("old family"),
        Verdict::Fails
    );
    assert_eq!(
        NumericItem
            .holds(Test::NumericGreaterThanOrEqual, &left, &right)
            .expect("numeric family"),
        Verdict::Holds
    );
}

#[test]
fn successor_accepts_only_exact_integer_data_and_rejects_malformed_text() {
    for (left, right, expected) in [
        (
            Datum::Number(WholeNumber(-1)),
            Datum::Text(TextView("0")),
            Verdict::Holds,
        ),
        (
            Datum::Text(TextView("9007199254740992")),
            Datum::Text(TextView("9007199254740993")),
            Verdict::Holds,
        ),
        (
            Datum::Text(TextView("1.0")),
            Datum::Number(WholeNumber(2)),
            Verdict::Fails,
        ),
        (
            Datum::Text(TextView("1")),
            Datum::Text(TextView("2.0")),
            Verdict::Fails,
        ),
        (
            Datum::Text(TextView("0xff")),
            Datum::Number(WholeNumber(256)),
            Verdict::Fails,
        ),
        (
            Datum::Flag(FlagState::Raised),
            Datum::Number(WholeNumber(2)),
            Verdict::Fails,
        ),
        (
            Datum::Text(TextView("1")),
            Datum::Text(TextView("NaN")),
            Verdict::Fails,
        ),
    ] {
        assert_eq!(
            NumericItem
                .holds(Test::IntegerSuccessor, &left, &right)
                .expect("successor family"),
            expected
        );
    }
}
