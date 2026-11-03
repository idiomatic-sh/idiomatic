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

#[test]
fn natural_text_comparison_is_separate_from_ordinary_ordering() {
    let left = Datum::Text(TextView("item2"));
    let right = Datum::Text(TextView("item10"));
    assert_eq!(
        OrderingItem.holds(Test::NaturalLessThan, &left, &right),
        Ok(Verdict::Holds)
    );
    assert_eq!(
        OrderingItem.holds(Test::LessThan, &left, &right),
        Ok(Verdict::Fails)
    );
    assert_eq!(
        OrderingItem.holds(
            Test::NaturalLessThan,
            &Datum::Number(WholeNumber(2)),
            &right
        ),
        Ok(Verdict::Fails)
    );
}

use super::item::natural_compare as compare;
use std::cmp::Ordering::{Equal, Less};

#[test]
fn numeric_runs_padding_boundaries_and_unicode_have_explicit_order() {
    for (left, right) in [
        ("item2", "item10"),
        ("item2a", "item10"),
        ("item1", "item1a"),
        ("x01", "x010"),
        ("x02", "x1"),
        ("x001", "x01"),
        ("A2", "a1"),
        ("a9", "a٩"),
        ("aé", "aΩ"),
        ("1 2", "12"),
        ("1a", "1b"),
    ] {
        assert_eq!(compare(left, right), Less, "{left:?} < {right:?}");
        assert_eq!(compare(right, left), Less.reverse());
    }
    for (left, right) in [
        ("a1", "a 1"),
        ("a\u{2003}1", "a1"),
        ("", " \t"),
        ("1 2", "1  2"),
    ] {
        assert_eq!(compare(left, right), Equal);
    }
    let long = "9".repeat(4096);
    let longer = format!("1{}", "0".repeat(4096));
    assert_eq!(compare(&long, &longer), Less);
    assert_eq!(compare("é", "e\u{301}"), Less.reverse());
}

#[test]
fn antisymmetry_transitivity_and_equivalent_whitespace_are_consistent() {
    let corpus = [
        "", " ", "0", "00", "01", "1", "1 2", "12", "2", "10", "a", "a1", "a 1", "a01", "a1a",
        "a2", "A1", "é", "e\u{301}", "٩",
    ];
    for a in corpus {
        for b in corpus {
            assert_eq!(compare(a, b), compare(b, a).reverse(), "{a:?}, {b:?}");
            for c in corpus {
                if compare(a, b).is_le() && compare(b, c).is_le() {
                    assert!(compare(a, c).is_le(), "{a:?} <= {b:?} <= {c:?}");
                }
                if compare(a, b) == Equal {
                    assert_eq!(compare(a, c), compare(b, c), "equivalent {a:?}, {b:?}");
                }
            }
        }
    }
}
