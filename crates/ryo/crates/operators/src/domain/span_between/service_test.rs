//! Both located operands are validated before an absent span propagates; the composition keeps the authored operand order.
use super::service::between;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::{PortableOperatorFault, SourceByteLength};
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{Integer, Offset, Span, Value as Cell},
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
type Input = (Vec<Cell>, u32, u32, usize);
type Value = Cell;
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
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("forward_endpoints_use_authored_operand_order"),
        word: Word::Succeeds,
        input: || (vec![span(4, 9), span(0, 7)], 0, 1, 9),
        expect: Expect::Value(span(4, 7)),
    },
    Case {
        name: CaseName("reversed_operands_compose_the_other_way"),
        word: Word::Succeeds,
        input: || (vec![span(4, 9), span(0, 7)], 1, 0, 9),
        expect: Expect::Value(span(0, 9)),
    },
    Case {
        name: CaseName("empty_span_at_zero_length_source"),
        word: Word::Succeeds,
        input: || (vec![span(0, 0), span(0, 0)], 0, 1, 0),
        expect: Expect::Value(span(0, 0)),
    },
    Case {
        name: CaseName("byte_intervals_need_not_begin_on_a_character_boundary"),
        word: Word::Succeeds,
        input: || (vec![span(1, 2), span(2, 3)], 0, 1, "λx".len()),
        expect: Expect::Value(span(1, 3)),
    },
    Case {
        name: CaseName("both_absent_spans_stay_absent"),
        word: Word::Succeeds,
        input: || (vec![Cell::Span(NO_SPAN), Cell::Span(NO_SPAN)], 0, 1, 0),
        expect: Expect::Value(Cell::Span(NO_SPAN)),
    },
    Case {
        name: CaseName("absent_left_with_valid_right_stays_absent"),
        word: Word::Succeeds,
        input: || (vec![Cell::Span(NO_SPAN), span(0, 3)], 0, 1, 3),
        expect: Expect::Value(Cell::Span(NO_SPAN)),
    },
    Case {
        name: CaseName("absent_right_with_valid_left_stays_absent"),
        word: Word::Succeeds,
        input: || (vec![span(0, 3), Cell::Span(NO_SPAN)], 0, 1, 3),
        expect: Expect::Value(Cell::Span(NO_SPAN)),
    },
    Case {
        name: CaseName("source_length_boundary_is_inclusive"),
        word: Word::Succeeds,
        input: || (vec![span(0, 2), span(1, 3)], 0, 1, 3),
        expect: Expect::Value(span(0, 3)),
    },
    Case {
        name: CaseName("largest_offsets_never_wrap_when_the_source_holds_them"),
        word: Word::Succeeds,
        input: || {
            (
                vec![span(u32::MAX, u32::MAX), span(u32::MAX, u32::MAX)],
                0,
                1,
                usize::MAX,
            )
        },
        expect: Expect::Value(span(u32::MAX, u32::MAX)),
    },
    Case {
        name: CaseName("absent_left_still_validates_an_overlong_right"),
        word: Word::Refuses,
        input: || (vec![Cell::Span(NO_SPAN), span(0, 4)], 0, 1, 3),
        expect: Expect::Error(PortableOperatorFault::InvalidSpan),
    },
    Case {
        name: CaseName("absent_left_still_validates_an_inverted_right"),
        word: Word::Refuses,
        input: || (vec![Cell::Span(NO_SPAN), span(3, 2)], 0, 1, 3),
        expect: Expect::Error(PortableOperatorFault::InvalidSpan),
    },
    Case {
        name: CaseName("absent_right_still_validates_an_overlong_left"),
        word: Word::Refuses,
        input: || (vec![span(0, 4), Cell::Span(NO_SPAN)], 0, 1, 3),
        expect: Expect::Error(PortableOperatorFault::InvalidSpan),
    },
    Case {
        name: CaseName("absent_right_still_validates_an_inverted_left"),
        word: Word::Refuses,
        input: || (vec![span(3, 2), Cell::Span(NO_SPAN)], 0, 1, 3),
        expect: Expect::Error(PortableOperatorFault::InvalidSpan),
    },
    Case {
        name: CaseName("composition_that_reverses_its_interval_is_refused"),
        word: Word::Refuses,
        input: || (vec![span(7, 8), span(0, 6)], 0, 1, 8),
        expect: Expect::Error(PortableOperatorFault::InvalidSpan),
    },
    Case {
        name: CaseName("source_length_one_short_is_refused"),
        word: Word::Refuses,
        input: || (vec![span(0, 2), span(1, 3)], 0, 1, 2),
        expect: Expect::Error(PortableOperatorFault::InvalidSpan),
    },
    Case {
        name: CaseName("largest_offsets_exceed_an_empty_source"),
        word: Word::Refuses,
        input: || {
            (
                vec![span(u32::MAX, u32::MAX), span(u32::MAX, u32::MAX)],
                0,
                1,
                0,
            )
        },
        expect: Expect::Error(PortableOperatorFault::InvalidSpan),
    },
    Case {
        name: CaseName("missing_right_column_is_reported_by_index"),
        word: Word::Refuses,
        input: || (vec![number(1)], 0, 1, 0),
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ryo_values::domain::plan_parts::types::ColumnIndex(1),
        }),
    },
    Case {
        name: CaseName("lookups_precede_the_type_check"),
        word: Word::Refuses,
        input: || (vec![number(1), Cell::Span(NO_SPAN)], 0, 1, 0),
        expect: Expect::Error(PortableOperatorFault::ShapeMismatch),
    },
    Case {
        name: CaseName("leftmost_missing_column_is_reported_first"),
        word: Word::Refuses,
        input: || (vec![], 8, 9, 0),
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ryo_values::domain::plan_parts::types::ColumnIndex(8),
        }),
    },
    Case {
        name: CaseName("inverted_left_does_not_hide_a_wrong_right_type"),
        word: Word::Refuses,
        input: || (vec![span(3, 2), number(1)], 0, 1, 3),
        expect: Expect::Error(PortableOperatorFault::ShapeMismatch),
    },
];
fn subject((row, start, end, length): Input) -> Result<Value, Failure> {
    let original = row.clone();
    let result = between(
        &row,
        ColumnIndex(start),
        ColumnIndex(end),
        SourceByteLength(length),
    );
    assert_eq!(row, original, "composition never mutates its input");
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
