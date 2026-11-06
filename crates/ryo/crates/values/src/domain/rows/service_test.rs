//! A row answers the cell at a column, and refuses a column its table does not have.
use super::errors::RowsError;
use super::service::cell;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::plan_parts::types::ColumnIndex;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};

#[derive(Clone, Copy)]
enum Scenario {
    SecondColumn,
    PastTheLastColumn,
}
type Input = Scenario;
type Value = Cell;
type Failure = RowsError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_cell_at_a_column_is_the_value_there"),
        word: Word::Succeeds,
        input: || Scenario::SecondColumn,
        expect: Expect::Value(Cell::Number(Integer(8))),
    },
    Case {
        name: CaseName("a_column_past_the_row_is_refused"),
        word: Word::Refuses,
        input: || Scenario::PastTheLastColumn,
        expect: Expect::Error(RowsError::NoSuchColumn { index: 2 }),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let row = [Cell::Number(Integer(7)), Cell::Number(Integer(8))];
    match input {
        Scenario::SecondColumn => cell(&row, ColumnIndex(1)),
        Scenario::PastTheLastColumn => cell(&row, ColumnIndex(2)),
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
