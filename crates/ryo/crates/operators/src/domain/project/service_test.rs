//! Projection's ordered bag and first-error cases.
use super::service::project;
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    DuplicateColumns,
    FirstMissingColumn,
}
type Input = Scenario;
type Value = [Cell; 3];
type Failure = PortableOperatorFault;
const ROW: [Cell; 2] = [Cell::Number(Integer(7)), Cell::Number(Integer(9))];
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("duplicate_columns_preserve_order"),
        word: Word::Succeeds,
        input: || Scenario::DuplicateColumns,
        expect: Expect::Value([ROW[1], ROW[0], ROW[1]]),
    },
    Case {
        name: CaseName("first_missing_column_is_reported"),
        word: Word::Refuses,
        input: || Scenario::FirstMissingColumn,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: 3 }),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let columns: &[ColumnIndex] = match input {
        Scenario::DuplicateColumns => &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
        Scenario::FirstMissingColumn => &[ColumnIndex(0), ColumnIndex(3), ColumnIndex(2)],
    };
    let mut output = Vec::new();
    project(&ROW, columns, &mut output)?;
    Ok(output.try_into().expect("three columns were selected"))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
