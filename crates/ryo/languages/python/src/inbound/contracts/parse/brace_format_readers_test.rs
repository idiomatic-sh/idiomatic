//! Regression cases for brace format readers bindings.
use super::{BraceFormatFieldItem, BraceFormatParseItem};
use crate::domain::relations::collections::relations::fixtures::contract::check;
use crate::fixtures::reading::reading;
use ryo::languages::contract::Want;
use crate::cases::{Case, CaseName, Expect, Word, run};

fn parse_rows_preserve_empty_valid_malformed_and_dynamic_states() {
    check::<BraceFormatParseItem>();
    let report = reading::<BraceFormatParseItem>(concat!(
        "a = '{{}}'\n",
        "b = '{name} {} {2}'\n",
        "c = '{name..bad}'\n",
        "d = f'{dynamic}'\n",
    ));
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(report.rows.len(), 4);
    let outcomes = report.rows.iter().map(|row| row[2]).collect::<Vec<_>>();
    assert_eq!(
        outcomes,
        [
            Want::Name("parsed"),
            Want::Name("parsed"),
            Want::Name("malformed"),
            Want::Name("unknown-input")
        ]
    );
    assert!(report.rows[0].len() > 4);
    assert!(report.rows[1].len() > 6);
    assert_eq!(report.rows[0][4], Want::Number(0));
    assert_eq!(report.rows[1][4], Want::Number(1));
    assert_eq!(report.rows[1][5], Want::Number(1));
    assert_eq!(report.rows[1][6], Want::Number(1));
    assert_eq!(report.rows[2][3], Want::Name("empty-attribute"));
}

fn field_rows_keep_duplicates_and_nested_parent_order() {
    check::<BraceFormatFieldItem>();
    let report = reading::<BraceFormatFieldItem>("'{key:{width}} {key}'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(report.rows.len(), 3);
    assert!(report.rows[0].len() > 2);
    assert!(report.rows[1].len() > 3);
    assert!(report.rows[2].len() > 7);
    assert_eq!(report.rows[0][1], Want::Number(0));
    assert_eq!(report.rows[0][2], Want::Name("literal"));
    assert_eq!(report.rows[1][1], Want::Number(1));
    assert_eq!(report.rows[1][2], Want::Name("format-spec"));
    assert_eq!(report.rows[1][3], Want::Number(0));
    assert_eq!(report.rows[2][1], Want::Number(2));
    assert_eq!(report.rows[2][7], Want::Name("key"));
}

fn oversized_positional_selector_keeps_exact_digits_and_overflow_flag() {
    let report = reading::<BraceFormatFieldItem>("'{9223372036854775808}'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    if usize::BITS >= i64::BITS {
        assert_eq!(report.rows.len(), 1);
        assert!(report.rows[0].len() > 7);
        assert_eq!(report.rows[0][5], Want::Number(0));
        assert_eq!(report.rows[0][6], Want::Flag(true));
        assert_eq!(report.rows[0][7], Want::Name("9223372036854775808"));
    } else {
        assert!(report.rows.is_empty());
    }
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `parse_rows_preserve_empty_valid_malformed_and_dynamic_states`.
    ParseRowsPreserveEmptyValidMalformedAndDynamicStates,
    /// The case `field_rows_keep_duplicates_and_nested_parent_order`.
    FieldRowsKeepDuplicatesAndNestedParentOrder,
    /// The case `oversized_positional_selector_keeps_exact_digits_and_overflow_flag`.
    OversizedPositionalSelectorKeepsExactDigitsAndOverflowFlag,
}

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("parse_rows_preserve_empty_valid_malformed_and_dynamic_states"),
        word: Word::Succeeds,
        input: || Body::ParseRowsPreserveEmptyValidMalformedAndDynamicStates,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("field_rows_keep_duplicates_and_nested_parent_order"),
        word: Word::Succeeds,
        input: || Body::FieldRowsKeepDuplicatesAndNestedParentOrder,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("oversized_positional_selector_keeps_exact_digits_and_overflow_flag"),
        word: Word::Succeeds,
        input: || Body::OversizedPositionalSelectorKeepsExactDigitsAndOverflowFlag,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::ParseRowsPreserveEmptyValidMalformedAndDynamicStates => parse_rows_preserve_empty_valid_malformed_and_dynamic_states(),
        Body::FieldRowsKeepDuplicatesAndNestedParentOrder => field_rows_keep_duplicates_and_nested_parent_order(),
        Body::OversizedPositionalSelectorKeepsExactDigitsAndOverflowFlag => oversized_positional_selector_keeps_exact_digits_and_overflow_flag(),
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, body);
}
