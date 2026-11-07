//! The `table_model` service's cases for copying rows into a table, on values alone.

use crate::domain::table_model::fixtures::take_read_row_counts;
use crate::domain::table_model::service::{count_of, materialize, read_row};
use crate::domain::table_model::types::{Rows, SelectedRows, StoredSelection};
use ryo_relations::domain::relations::service::to_bits;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value as Cell,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::{RowIndex, Table};
use std::sync::Arc;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, RowsError>;
/// What a scenario answers: a yes as one.
type Value = usize;
/// Why a scenario is refused.
type Failure = RowsError;

fn compact_materialization_decodes_projected_cells_without_full_row_reads() -> Result<Value, Failure> {
    let physical = [
        FactColumn(
            [1, 2, 3]
                .map(|value| to_bits(Cell::Number(Integer(value))).0)
                .to_vec(),
        ),
        FactColumn(
            [8, 9]
                .map(|value| to_bits(Cell::Number(Integer(value))).0)
                .to_vec(),
        ),
    ];
    let selected = Rows::Selected(SelectedRows {
        source: ColumnsView {
            kinds: &[CellKind::Number, CellKind::Number],
            columns: &physical,
        },
        selection: Arc::new(StoredSelection {
            relation: Relation::Function,
            rows: Arc::new(vec![1, 0, 1, 2]),
            columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
            kinds: vec![CellKind::Number; 3],
            identity: Arc::new(Default::default()),
        }),
    });
    // The fourth physical row is malformed. Its first projected cell is absent,
    // so both paths omit it while retaining duplicate logical positions.
    let _previous = take_read_row_counts();
    let mut actual = Table::default();
    materialize(&selected, &mut actual, &mut Vec::new());
    assert_eq!(take_read_row_counts(), (0, 0));
    assert_eq!(actual.count.0, 3);
    assert_eq!(
        actual.cells,
        vec![
            Cell::Number(Integer(9)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(9)),
            Cell::Number(Integer(8)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(8)),
            Cell::Number(Integer(9)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(9)),
        ]
    );
    let mut reference = Table {
        kinds: actual.kinds.clone(),
        ..Table::default()
    };
    let mut scratch = Vec::new();
    for index in 0..count_of(&selected).0 {
        if let Some(row) = read_row(&selected, RowIndex(index), &mut scratch) {
            push_row(&mut reference, row);
        }
    }
    assert_eq!(actual, reference);
    Ok(1)
}

fn compact_materialization_preserves_ragged_and_zero_width_legacy_rows() -> Result<Value, Failure> {
    let physical = [
        FactColumn(vec![
            to_bits(Cell::Number(Integer(1))).0,
            to_bits(Cell::Number(Integer(2))).0,
        ]),
        FactColumn(vec![to_bits(Cell::Number(Integer(9))).0]),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Number],
        columns: &physical,
    };
    let selection = |rows, columns, kinds| {
        Rows::Selected(SelectedRows {
            source,
            selection: Arc::new(StoredSelection {
                relation: Relation::Function,
                rows: Arc::new(rows),
                columns,
                kinds,
                identity: Arc::new(Default::default()),
            }),
        })
    };

    // An unused physical column may be ragged; only selected cells are read.
    let mut projected = Table::default();
    materialize(
        &selection(vec![1, 0], vec![ColumnIndex(0)], vec![CellKind::Number]),
        &mut projected,
        &mut Vec::new(),
    );
    assert_eq!(projected.count.0, 2);
    assert_eq!(
        projected.cells,
        vec![Cell::Number(Integer(2)), Cell::Number(Integer(1))]
    );

    // An invalid selected column skips its row without publishing partial cells.
    let mut invalid_column = Table::default();
    materialize(
        &selection(vec![0], vec![ColumnIndex(5)], vec![CellKind::Number]),
        &mut invalid_column,
        &mut Vec::new(),
    );
    assert_eq!(invalid_column.count.0, 0);
    assert!(invalid_column.cells.is_empty());

    // Legacy read_row did not revalidate a malformed declared kind. Retain the
    // same table shape and decoded source value until the admission layer rejects it.
    let mut invalid_kind = Table::default();
    materialize(
        &selection(vec![0], vec![ColumnIndex(0)], vec![CellKind::Symbol]),
        &mut invalid_kind,
        &mut Vec::new(),
    );
    assert_eq!(invalid_kind.kinds, vec![CellKind::Symbol]);
    assert_eq!(invalid_kind.cells, vec![Cell::Number(Integer(1))]);
    assert_eq!(invalid_kind.count.0, 1);

    // Zero-width projections retain each logical row, even when its physical
    // position is beyond a provider column that the projection never reads.
    let mut zero_width = Table::default();
    materialize(
        &selection(vec![3, 0, 3], vec![], vec![]),
        &mut zero_width,
        &mut Vec::new(),
    );
    assert_eq!(zero_width.count.0, 3);
    assert!(zero_width.kinds.is_empty());
    assert!(zero_width.cells.is_empty());
    Ok(1)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("compact_materialization_decodes_projected_cells_without_full_row_reads"),
        word: Word::Succeeds,
        input: || compact_materialization_decodes_projected_cells_without_full_row_reads,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("compact_materialization_preserves_ragged_and_zero_width_legacy_rows"),
        word: Word::Succeeds,
        input: || compact_materialization_preserves_ragged_and_zero_width_legacy_rows,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
