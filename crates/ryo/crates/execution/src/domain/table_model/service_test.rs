//! The `table_model` service's cases, on values alone.

use ryo_relations::domain::relations::types::{CellKind, FactColumn, Integer, Value as Cell};
use ryo_values::selects;

use crate::domain::table_model::collections::representations::types::{
    CellRead, PhysicalShape, RepresentationKind, RowRead,
};
use crate::domain::table_model::service::{borrowed, kind, registered};
use crate::domain::table_model::types::StoredSelection;
use ryo_relations::domain::relations::{service::to_bits, types::Relation};
use std::sync::Arc;

#[cfg(test)]
mod materialization_test;
#[cfg(test)]
mod physical_test;
#[cfg(test)]
mod representations_test;
#[cfg(test)]
mod rows_test;
#[cfg(test)]
mod stored_keys_test;

use super::types::Rows;
use crate::domain::positions::service_test::prepared_index;
use ryo_derived::domain::contract::types::On;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::errors::RowsError;
use {
    super::service::matching, ryo_values::domain::rows::service::cell,
    ryo_values::domain::rows::service::push_row,
};

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

/// Looking up a key finds the rows that have it, in the order they came, whatever the order of the table.
fn finds_the_rows_of_a_key() -> Result<Value, RowsError> {
    let table = pairs(&[[2, 1], [1, 2], [2, 3], [3, 4]]);
    let on = [ColumnIndex(0)];
    let order = prepared_index(&Rows::Computed(&table), &on)?;
    let found: Vec<RowIndex> = matching(
        &table,
        On::Columns(&on),
        &order,
        &[Cell::Number(Integer(2))],
    )
    .copied()
    .collect();
    let none = matching(
        &table,
        On::Columns(&on),
        &order,
        &[Cell::Number(Integer(9))],
    );
    Ok(usize::from(
        found == [RowIndex(0), RowIndex(2)] && none.clone().next().is_none(),
    ))
}

/// Among many keys, a key finds exactly the rows that have it, still in the order they came.
fn finds_a_key_among_many() -> Result<Value, RowsError> {
    let rows: Vec<[i64; 2]> = (0..700).map(|each| [each % 7, each]).collect();
    let table = pairs(&rows);
    let on = [ColumnIndex(0)];
    let order = prepared_index(&Rows::Computed(&table), &on)?;
    let found: Vec<RowIndex> = matching(
        &table,
        On::Columns(&on),
        &order,
        &[Cell::Number(Integer(3))],
    )
    .copied()
    .collect();
    let expected: Vec<RowIndex> = (0..100).map(|each| RowIndex(3 + 7 * each)).collect();
    Ok(usize::from(
        found == expected
            && matching(
                &table,
                On::Columns(&on),
                &order,
                &[Cell::Number(Integer(7))],
            )
            .next()
            .is_none(),
    ))
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
        name: CaseName("a_key_finds_its_rows_in_the_order_they_came"),
        word: Word::Succeeds,
        input: || finds_the_rows_of_a_key,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_key_among_many_finds_exactly_its_rows"),
        word: Word::Succeeds,
        input: || finds_a_key_among_many,
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
pub(crate) fn selection(
    rows: Vec<u32>,
    columns: Vec<ColumnIndex>,
    kinds: Vec<CellKind>,
) -> Arc<StoredSelection> {
    Arc::new(StoredSelection {
        relation: Relation::Function,
        rows: Arc::new(rows),
        columns,
        kinds,
        identity: Arc::new(Default::default()),
    })
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
                CellRead::Cell(*wanted)
            );
            assert_eq!(
                crate::domain::table_model::service::cell_at(input, RowIndex(at), at_column),
                Some(*wanted)
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
