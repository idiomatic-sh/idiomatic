//! The `set_equality` item's cases: a left row is absent from the right when no row of it has its key.

use ryo_language::domain::relations::types::{CellKind, Value as Cell};

use super::item::SetEqualityItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::collections::operators::antijoin::collections::strategies::interface::StrategiesItem as _;
use crate::domain::operator::collections::operators::antijoin::collections::strategies::types::Absence;
use crate::domain::positions::types::KeyColumns;
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{index, push_row};
use crate::domain::table_model::types::{KeyIndex, On, Table};
use ryo_language::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// What a row asks: the row on the left.
type Input = fn() -> Result<Absence, OperatingError>;
/// Whether the row is absent from the right.
type Value = Absence;
type Failure = OperatingError;

/// A table of one number a row: 1 and 2.
fn right() -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for number in [1, 2] {
        push_row(&mut table, &[Cell::Number(Integer(number))]);
    }
    table
}

fn absent(number: i64, left_on: u32) -> Result<Absence, OperatingError> {
    let table = right();
    let mut order = KeyIndex::default();
    let right_on = [ColumnIndex(0)];
    index(&table, On::Columns(&right_on), &mut order)?;
    let left = [ColumnIndex(left_on)];
    SetEqualityItem.absent(
        &table,
        &order,
        KeyColumns {
            left: &left,
            right: &right_on,
        },
        &[Cell::Number(Integer(number))],
        &mut Vec::new(),
    )
}

fn a_key_the_right_holds_is_not_absent() -> Result<Absence, OperatingError> {
    absent(2, 0)
}

fn a_key_the_right_lacks_is_absent() -> Result<Absence, OperatingError> {
    absent(9, 0)
}

fn a_row_without_the_column_is_refused() -> Result<Absence, OperatingError> {
    absent(1, 4)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_key_the_right_holds_is_not_absent"),
        word: Word::Succeeds,
        input: || a_key_the_right_holds_is_not_absent,
        expect: Expect::Value(Absence::Present),
    },
    Case {
        name: CaseName("a_key_the_right_lacks_is_absent"),
        word: Word::Succeeds,
        input: || a_key_the_right_lacks_is_absent,
        expect: Expect::Value(Absence::Absent),
    },
    Case {
        name: CaseName("a_row_without_the_column_is_refused"),
        word: Word::Refuses,
        input: || a_row_without_the_column_is_refused,
        expect: Expect::Error(OperatingError::Table(TableModelError::NoSuchColumn {
            index: 4,
        })),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
