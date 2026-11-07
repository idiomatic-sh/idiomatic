//! The `source_matches` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SourceMatchesItem;
use super::super::interface::OperatorsItem;
use super::source::{ValidatedSource, validate};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::types::{Integer, Offset, Relation, Span, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as Source};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::QueryOperator;

/// A line break between an accented letter and the end of the text.
const SPLIT: &[u8] = "xé\r\ny".as_bytes();
/// One accented letter, two bytes.
const ACCENT: &[u8] = "é".as_bytes();

/// What a row asks of the item.
#[derive(Clone, Copy)]
enum Asked {
    /// The shared cases of the collection.
    Describe,
    /// The gate over a span of the split text and a positive base line.
    Retains,
    /// The gate over an absent span and a base line column that does not exist.
    AbsentSpan,
    /// The gate over a span of the accented letter.
    Span(u32, u32),
    /// The gate over a whole span and a base line of zero.
    ZeroLine,
    /// Preparing an operation that is not this operator's.
    Foreign,
}

/// What a row answers.
#[derive(Debug, PartialEq)]
enum Found {
    /// The operator describes itself.
    Described,
    /// The physical text of the span, where it starts and the line it begins on.
    Gate {
        base: usize,
        text: &'static str,
        line: Option<i64>,
    },
    /// The span is absent, so no occurrence is looked for.
    Absent,
}

type Input = Asked;
type Value = Found;
type Failure = OperatingError;

fn span(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}

fn found(validated: Option<ValidatedSource<'static>>) -> Found {
    match validated {
        Some(gate) => Found::Gate {
            base: gate.base,
            text: gate.text,
            line: gate.line,
        },
        None => Found::Absent,
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    match asked {
        Asked::Describe => {
            check(QueryOperator::SourceMatches);
            Ok(Found::Described)
        }
        Asked::Retains => {
            let row = [span(1, 5), Cell::Number(Integer(7))];
            validate(&row, ColumnIndex(0), Some(ColumnIndex(1)), SPLIT).map(found)
        }
        Asked::AbsentSpan => validate(
            &[Cell::Span(NO_SPAN)],
            ColumnIndex(0),
            Some(ColumnIndex(99)),
            ACCENT,
        )
        .map(found),
        Asked::Span(start, end) => {
            validate(&[span(start, end)], ColumnIndex(0), None, ACCENT).map(found)
        }
        Asked::Foreign => SourceMatchesItem
            .prepare(
                &Operation::Union {
                    left: Source::Relation(Relation::Function),
                    right: Source::Relation(Relation::Function),
                },
                &[],
                &[],
            )
            .map(|_| Found::Absent),
        Asked::ZeroLine => validate(
            &[span(0, 2), Cell::Number(Integer(0))],
            ColumnIndex(0),
            Some(ColumnIndex(1)),
            ACCENT,
        )
        .map(found),
    }
}

/// The rows of this item's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_source_matches_operator_describes_itself"),
        word: Word::Succeeds,
        input: || Asked::Describe,
        expect: Expect::Value(Found::Described),
    },
    Case {
        name: CaseName("the_checked_gate_retains_physical_offsets_and_a_positive_line"),
        word: Word::Succeeds,
        input: || Asked::Retains,
        expect: Expect::Value(Found::Gate {
            base: 1,
            text: "é\r\n",
            line: Some(7),
        }),
    },
    Case {
        name: CaseName("an_absent_span_skips_the_line_check"),
        word: Word::Succeeds,
        input: || Asked::AbsentSpan,
        expect: Expect::Value(Found::Absent),
    },
    Case {
        name: CaseName("a_span_inside_a_character_fails"),
        word: Word::Refuses,
        input: || Asked::Span(1, 2),
        expect: Expect::Error(OperatingError::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("a_span_that_ends_before_it_starts_fails"),
        word: Word::Refuses,
        input: || Asked::Span(2, 1),
        expect: Expect::Error(OperatingError::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("a_span_past_the_text_fails"),
        word: Word::Refuses,
        input: || Asked::Span(0, 3),
        expect: Expect::Error(OperatingError::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("the_source_matches_item_refuses_an_operation_that_is_not_its_own"),
        word: Word::Refuses,
        input: || Asked::Foreign,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_base_line_of_zero_fails"),
        word: Word::Refuses,
        input: || Asked::ZeroLine,
        expect: Expect::Error(OperatingError::InvalidSourceMatchSpan),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
