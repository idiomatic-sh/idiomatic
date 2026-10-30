//! The `hash` item's cases: the rows of a table that have the key of a left row.

use ryo_language::domain::relations::types::{CellKind, Value as Cell};

use super::item::HashItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::positions::types::KeyColumns;
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{index, push_row};
use crate::domain::table_model::types::{KeyIndex, On, Table};
use crate::domain::tree_join::collections::strategies::interface::StrategiesItem as _;
use crate::domain::tree_join::collections::strategies::types::{Finding, Fitting};
use ryo_language::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// What a row asks: the key of the row on the left.
type Input = fn() -> Result<[usize; 4], OperatingError>;
/// The indexes of the rows found.
type Value = [usize; 4];
/// What pads a row of answers that found fewer than four.
const NONE: usize = usize::MAX;

/// The answers padded to four.
fn four(found: Vec<usize>) -> [usize; 4] {
    let mut padded = [NONE; 4];
    for (slot, each) in padded.iter_mut().zip(found) {
        *slot = each;
    }
    padded
}
type Failure = OperatingError;

/// A table of two numbers a row: (1, 10), (2, 20), (1, 30).
fn table() -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number, CellKind::Number],
        ..Table::default()
    };
    for row in [[1, 10], [2, 20], [1, 30]] {
        push_row(&mut table, &row.map(|number| Cell::Number(Integer(number))));
    }
    table
}

fn found(row: &[Cell], left_on: u32) -> Result<[usize; 4], OperatingError> {
    let table = table();
    let mut order = KeyIndex::default();
    let right_on = [ColumnIndex(0)];
    index(&table, On::Columns(&right_on), &mut order)?;
    let mut key = Vec::new();
    let left = [ColumnIndex(left_on)];
    Ok(four(
        HashItem::find(
            &table,
            &order,
            KeyColumns {
                left: &left,
                right: &right_on,
            },
            row,
            &mut key,
        )?
        .map(|each| each.0)
        .collect(),
    ))
}

fn a_key_finds_the_rows_that_have_it_in_the_order_they_came() -> Result<[usize; 4], OperatingError>
{
    found(&[Cell::Number(Integer(1))], 0)
}

fn a_key_no_row_has_finds_nothing() -> Result<[usize; 4], OperatingError> {
    found(&[Cell::Number(Integer(9))], 0)
}

fn a_row_without_the_column_is_refused() -> Result<[usize; 4], OperatingError> {
    found(&[Cell::Number(Integer(1))], 3)
}

fn it_applies_where_no_finding_is_in_a_tree() -> [usize; 4] {
    four(vec![
        usize::from(HashItem.applies(None) == Fitting::Fits),
        usize::from(HashItem.applies(Some(Finding::Span(ColumnIndex(0)))) == Fitting::Fits),
    ])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_key_finds_the_rows_that_have_it_in_the_order_they_came"),
        word: Word::Succeeds,
        input: || a_key_finds_the_rows_that_have_it_in_the_order_they_came,
        expect: Expect::Value([0, 2, NONE, NONE]),
    },
    Case {
        name: CaseName("a_key_no_row_has_finds_nothing"),
        word: Word::Succeeds,
        input: || a_key_no_row_has_finds_nothing,
        expect: Expect::Value([NONE, NONE, NONE, NONE]),
    },
    Case {
        name: CaseName("it_applies_where_no_finding_is_in_a_tree"),
        word: Word::Succeeds,
        input: || || Ok(it_applies_where_no_finding_is_in_a_tree()),
        expect: Expect::Value([1, 0, NONE, NONE]),
    },
    Case {
        name: CaseName("a_row_without_the_column_is_refused"),
        word: Word::Refuses,
        input: || a_row_without_the_column_is_refused,
        expect: Expect::Error(OperatingError::Table(TableModelError::NoSuchColumn {
            index: 3,
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
