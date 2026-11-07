//! The `predicate` service's cases, on values alone.

use super::service::{distinct, matches, postings};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use crate::domain::positions::types::RowSlots;
use crate::domain::row_cells::types::{PredicateRows, PredicateVerdict};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::{ColumnIndex, LiteralText};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    EvenRowsHold,
    CandidatesSelectRows,
    UnorderedCandidates,
    DistinctKeepsFirstOfEachKey,
    UnknownLiteral,
}
type Input = Scenario;
type Value = Cow<'static, [u32]>;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_rows_that_hold_are_listed_in_order"),
        word: Word::Succeeds,
        input: || Scenario::EvenRowsHold,
        expect: Expect::Value(Cow::Borrowed(&[0, 2])),
    },
    Case {
        name: CaseName("candidates_limit_the_rows_decided"),
        word: Word::Succeeds,
        input: || Scenario::CandidatesSelectRows,
        expect: Expect::Value(Cow::Borrowed(&[2])),
    },
    Case {
        name: CaseName("distinct_keeps_the_first_row_of_each_key"),
        word: Word::Succeeds,
        input: || Scenario::DistinctKeepsFirstOfEachKey,
        expect: Expect::Value(Cow::Borrowed(&[0, 1])),
    },
    Case {
        name: CaseName("candidates_out_of_order_are_refused"),
        word: Word::Refuses,
        input: || Scenario::UnorderedCandidates,
        expect: Expect::Error(DerivedFault::ShapeMismatch),
    },
    Case {
        name: CaseName("an_active_literal_outside_the_allowed_set_is_refused"),
        word: Word::Refuses,
        input: || Scenario::UnknownLiteral,
        expect: Expect::Error(DerivedFault::WrongOperation),
    },
];

struct EvenRows(RowCount);
impl PredicateRows for EvenRows {
    fn count(&self) -> RowCount {
        self.0
    }
    fn holds(&mut self, row: RowIndex) -> Result<PredicateVerdict, DerivedFault> {
        Ok(if row.0 % 2 == 0 {
            PredicateVerdict::Holds
        } else {
            PredicateVerdict::Fails
        })
    }
}

fn positions(found: &crate::domain::sharing::types::SharedPositions) -> Value {
    Cow::Owned(found.0.as_slice().to_vec())
}

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let keys = Table {
        kinds: vec![CellKind::Number],
        cells: [1, 2, 1, 2].map(|key| Cell::Number(Integer(key))).to_vec(),
        count: RowCount(4),
    };
    match input {
        Scenario::EvenRowsHold => Ok(positions(&matches(&mut EvenRows(RowCount(4)), None)?)),
        Scenario::CandidatesSelectRows => Ok(positions(&matches(
            &mut EvenRows(RowCount(4)),
            Some(RowSlots(&[1, 2])),
        )?)),
        Scenario::UnorderedCandidates => Ok(positions(&matches(
            &mut EvenRows(RowCount(4)),
            Some(RowSlots(&[2, 1])),
        )?)),
        Scenario::DistinctKeepsFirstOfEachKey => Ok(positions(&distinct(
            &keys,
            &[ColumnIndex(0)],
            RowSlots(&[0, 1, 2, 3]),
        )?)),
        Scenario::UnknownLiteral => postings(
            &keys,
            ColumnIndex(0),
            &[LiteralText("a".into())],
            &[LiteralText("b".into())],
            &Symbols::default(),
            ByteSlice(b""),
        )
        .map(|_| Cow::Borrowed(&[][..])),
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
