//! The `table_model` service's cases, on values alone.

use ryo_relations::domain::relations::types::{CellKind, FactColumn, Integer, Value as Cell};
use ryo_values::selects;

use crate::domain::table_model::collections::representations::types::{CellRead, RowRead};
use crate::domain::table_model::service::{borrowed, kind, registered};
use crate::domain::table_model::types::{PhysicalShape, RepresentationKind};
use ryo_relations::domain::relations::service::to_bits;
use std::sync::Arc;

#[cfg(test)]
mod physical_test;
#[cfg(test)]
mod representations_test;
#[cfg(test)]
mod rows_test;

use super::types::Rows;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::errors::RowsError;
use {ryo_values::domain::rows::service::cell, ryo_values::domain::rows::service::push_row};

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, RowsError>;
/// What a scenario answers: a yes as one, or a count.
type Value = usize;
/// Why a scenario is refused.
type Failure = RowsError;

fn finds_the_second_cell() -> Result<Value, RowsError> {
    let row = [Cell::Number(Integer(1)), Cell::Number(Integer(2))];
    Ok(usize::from(
        cell(&row, ColumnIndex(1))? == Cell::Number(Integer(2)),
    ))
}

fn misses_a_cell() -> Result<Value, RowsError> {
    cell(&[Cell::Number(Integer(1))], ColumnIndex(4)).map(|_| 0)
}

/// A table of two numbers a row.
pub(crate) fn pairs(rows: &[[i64; 2]]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    for [left, right] in rows {
        push_row(
            &mut table,
            &[Cell::Number(Integer(*left)), Cell::Number(Integer(*right))],
        );
    }
    table
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_cell_is_found_by_position"),
        word: Word::Succeeds,
        input: || finds_the_second_cell,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_position_the_row_lacks_is_refused"),
        word: Word::Refuses,
        input: || misses_a_cell,
        expect: Expect::Refusal(|error| selects!(matches!(error, RowsError::NoSuchColumn { .. }))),
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

use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

pub(crate) fn compact(table: &Table) -> ryo_values::domain::table_words::types::CompactTable {
    use ryo_relations::domain::relations::types::Words;
    use ryo_values::domain::stored_facts::types::KeptWords;
    use ryo_values::domain::table_words::{
        service,
        types::{DecodeMode, WordSlice},
    };
    let mut words = Words::default();
    service::encode_table(table, &[], &mut words);
    let ryo_values::domain::table_words::types::Admitted {
        table: compact,
        names,
    } = ryo_values::domain::compact_table::service::admit_compact(
        ryo_values::domain::table_words::types::CompactBacking(Arc::new(KeptWords(service::pack(
            WordSlice(&words.0),
        )))),
        &mut Vec::new(),
        DecodeMode::Complete,
    )
    .unwrap();
    assert!(names.is_empty());
    assert_eq!(
        ryo_values::domain::compact_table::service::compact_rectangular(&compact),
        ryo_values::domain::table_words::types::TableShape::Rectangular
    );
    compact
}

pub(crate) fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}
pub(crate) fn column(values: &[i64]) -> FactColumn {
    FactColumn(
        values
            .iter()
            .map(|value| to_bits(number(*value)).0)
            .collect(),
    )
}
pub(crate) fn values(read: RowRead<'_>) -> &[Cell] {
    match read {
        RowRead::Borrowed(row) | RowRead::Decoded(row) => row,
        RowRead::MissingPhysical => panic!("declared row missing"),
    }
}
pub(crate) fn check_rows(input: &Rows<'_>, expected: &[Vec<Cell>]) {
    let raw = borrowed(input);
    let item = registered(kind(raw));
    let contract = item.prepare(raw).expect("metadata");
    assert_eq!(item.inspect(&contract).unwrap(), PhysicalShape::Complete);
    assert_eq!(contract.rows, RowCount(expected.len()));
    assert_eq!(kind(item.bulk(raw).unwrap()), contract.kind);
    for (at, wanted) in expected.iter().enumerate() {
        let mut decode = vec![number(99); contract.kinds.len()];
        assert_eq!(
            values(item.row(raw, RowIndex(at), &mut decode).unwrap()),
            wanted
        );
        let mut decode = Vec::new();
        assert_eq!(
            crate::domain::table_model::service::read_row(input, RowIndex(at), &mut decode)
                .unwrap(),
            wanted
        );
        for (at_column, wanted) in wanted.iter().enumerate() {
            let at_column = ColumnIndex(u32::try_from(at_column).unwrap());
            assert_eq!(
                item.cell(raw, RowIndex(at), at_column).unwrap(),
                CellRead::Value(*wanted)
            );
        }
    }
    // A zero-column Stored view has no physical row bound; callers enumerate
    // only the zero rows declared by its contract, preserving legacy behavior.
    if contract.rows != RowCount(0) || contract.kind != RepresentationKind::Stored {
        let mut decode = vec![number(0); contract.kinds.len()];
        assert!(matches!(
            item.row(raw, RowIndex(usize::MAX), &mut decode).unwrap(),
            RowRead::MissingPhysical
        ));
    }
    assert_eq!(
        item.cell(raw, RowIndex(0), ColumnIndex(u32::MAX)).unwrap(),
        CellRead::MissingPhysical
    );
}
