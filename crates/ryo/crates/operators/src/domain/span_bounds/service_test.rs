//! Endpoint extraction keeps the order of the span and never mutates its row.
use super::service::bounds;
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{Integer, Offset, Span, Value as Cell},
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
type Input = (Vec<Cell>, u32);
type Value = [Cell; 2];
type Failure = PortableOperatorFault;
const fn span(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}
const fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}
/// The largest span offset, as a number.
const LARGEST: i64 = 4_294_967_295;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("endpoints_are_the_start_and_end_of_the_span"),
        word: Word::Succeeds,
        input: || (vec![number(9), span(4, 7)], 1),
        expect: Expect::Value([number(4), number(7)]),
    },
    Case {
        name: CaseName("empty_span_has_equal_endpoints"),
        word: Word::Succeeds,
        input: || (vec![number(1), span(0, 0)], 1),
        expect: Expect::Value([number(0), number(0)]),
    },
    Case {
        name: CaseName("duplicate_rows_answer_alike"),
        word: Word::Succeeds,
        input: || (vec![number(9), span(4, 7)], 1),
        expect: Expect::Value([number(4), number(7)]),
    },
    Case {
        name: CaseName("largest_offsets_are_checked_without_a_source_length_gate"),
        word: Word::Succeeds,
        input: || (vec![span(u32::MAX, u32::MAX)], 0),
        expect: Expect::Value([number(LARGEST), number(LARGEST)]),
    },
    Case {
        name: CaseName("missing_column_is_reported_by_index"),
        word: Word::Refuses,
        input: || (vec![], 7),
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: 7 }),
    },
    Case {
        name: CaseName("a_number_is_not_a_span"),
        word: Word::Refuses,
        input: || (vec![number(2)], 0),
        expect: Expect::Error(PortableOperatorFault::ShapeMismatch),
    },
    Case {
        name: CaseName("the_absent_span_has_no_bounds"),
        word: Word::Refuses,
        input: || (vec![Cell::Span(NO_SPAN)], 0),
        expect: Expect::Error(PortableOperatorFault::InvalidSpanBounds),
    },
    Case {
        name: CaseName("an_inverted_span_has_no_bounds"),
        word: Word::Refuses,
        input: || (vec![span(6, 5)], 0),
        expect: Expect::Error(PortableOperatorFault::InvalidSpanBounds),
    },
];
fn subject((row, column): Input) -> Result<Value, Failure> {
    let original = row.clone();
    let result = bounds(&row, ColumnIndex(column));
    assert_eq!(row, original, "extraction never mutates its row");
    result
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
