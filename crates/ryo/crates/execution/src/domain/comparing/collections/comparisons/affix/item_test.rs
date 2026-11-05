//! The `affix` item's cases.

use super::super::interface::ComparisonsItem as _;
use super::item::AffixItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::domain::plan_parts::types::Test;

/// A comparison: the test, the left side and the right side.
type Input = (Test, Datum<'static>, Datum<'static>);
type Value = Verdict;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_text_starts_with_its_prefix"),
        word: Word::Succeeds,
        input: || {
            (
                Test::StartsWith,
                Datum::Text(TextView("TodosService")),
                Datum::Text(TextView("Todos")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_text_does_not_start_with_another_text"),
        word: Word::Succeeds,
        input: || {
            (
                Test::StartsWith,
                Datum::Text(TextView("TodosService")),
                Datum::Text(TextView("Service")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("a_text_ends_with_its_suffix"),
        word: Word::Succeeds,
        input: || {
            (
                Test::EndsWith,
                Datum::Text(TextView("TodosService")),
                Datum::Text(TextView("Service")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_text_does_not_end_with_another_text"),
        word: Word::Succeeds,
        input: || {
            (
                Test::EndsWith,
                Datum::Text(TextView("TodosService")),
                Datum::Text(TextView("Todos")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("every_text_starts_and_ends_with_empty_text"),
        word: Word::Succeeds,
        input: || {
            (
                Test::EndsWith,
                Datum::Text(TextView("Todos")),
                Datum::Text(TextView("")),
            )
        },
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_shorter_text_does_not_end_with_a_longer_one"),
        word: Word::Succeeds,
        input: || {
            (
                Test::EndsWith,
                Datum::Text(TextView("Service")),
                Datum::Text(TextView("TodosService")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("a_side_that_is_not_text_does_not_match"),
        word: Word::Succeeds,
        input: || {
            (
                Test::StartsWith,
                Datum::Unlocated,
                Datum::Text(TextView("")),
            )
        },
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("a_prefix_that_is_not_text_does_not_match"),
        word: Word::Succeeds,
        input: || {
            (
                Test::EndsWith,
                Datum::Text(TextView("Todos")),
                Datum::Unlocated,
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
                Datum::Text(TextView("a")),
                Datum::Text(TextView("a")),
            )
        },
        expect: Expect::Error(OperatingError::WrongTest),
    },
    Case {
        name: CaseName("a_test_of_another_family_is_refused_whatever_its_sides"),
        word: Word::Refuses,
        input: || (Test::GlobAny, Datum::Unlocated, Datum::Text(TextView("a"))),
        expect: Expect::Error(OperatingError::WrongTest),
    },
];

fn subject((test, left, right): Input) -> Result<Value, Failure> {
    AffixItem.holds(test, &left, &right)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
