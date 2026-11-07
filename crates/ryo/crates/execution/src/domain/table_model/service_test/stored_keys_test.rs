//! The `table_model` service's cases for the keys of stored rows, on values alone.

use crate::domain::table_cells::fixtures::prepared_index;
use crate::domain::table_model::service::{matching, matching_stored, materialize, rectangular};
use crate::domain::table_model::types::Rows;
use ryo_derived::domain::key::types::On;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Flag, Integer, Value as Cell,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::types::{RowIndex, Table};
use ryo_values::domain::table_words::types::TableShape;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, RowsError>;
/// What a scenario answers: a yes as one.
type Value = usize;
/// Why a scenario is refused.
type Failure = RowsError;

fn stored_keys_preserve_order_duplicates_and_typed_flag_normalization() -> Result<Value, Failure> {
    const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Flag, CellKind::Number];
    let columns = [
        FactColumn(vec![2, 1, 2, 2]),
        FactColumn(vec![1, 0, 2, 0]),
        FactColumn(vec![20, 10, 21, 22]),
    ];
    let view = ColumnsView {
        kinds: KINDS,
        columns: &columns,
    };
    assert_eq!(rectangular(view), TableShape::Rectangular);
    let mut materialized = Table::default();
    materialize(&Rows::Stored(view), &mut materialized, &mut Vec::new());
    for key_columns in [
        vec![ColumnIndex(0)],
        vec![ColumnIndex(1), ColumnIndex(0)],
        Vec::new(),
    ] {
        let stored = prepared_index(&Rows::Stored(view), &key_columns).expect("valid stored key");
        let computed = prepared_index(&Rows::Computed(&materialized), &key_columns)
            .expect("valid computed key");
        for row in 0..materialized.count.0 {
            let row = ryo_values::domain::rows::service::row_in(&materialized, RowIndex(row))
                .expect("row");
            let key = key_columns
                .iter()
                .map(|column| {
                    ryo_values::domain::rows::service::cell(row, *column).expect("key column")
                })
                .collect::<Vec<_>>();
            assert_eq!(
                matching_stored(
                    view,
                    &key_columns,
                    &stored,
                    &key,
                    #[cfg(feature = "diagnostic-row-progress")]
                    &crate::domain::operator::contract::views::IgnoredProgress,
                )
                .copied()
                .collect::<Vec<_>>(),
                matching(&materialized, On::Columns(&key_columns), &computed, &key)
                    .copied()
                    .collect::<Vec<_>>()
            );
        }
    }
    let stored = prepared_index(&Rows::Stored(view), &[ColumnIndex(1)]).expect("flag index");
    assert_eq!(
        matching_stored(
            view,
            &[ColumnIndex(1)],
            &stored,
            &[Cell::Flag(Flag::Raised)],
            #[cfg(feature = "diagnostic-row-progress")]
            &crate::domain::operator::contract::views::IgnoredProgress,
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(2)]
    );
    assert!(
        matching_stored(
            view,
            &[ColumnIndex(1)],
            &stored,
            &[Cell::Number(Integer(1))],
            #[cfg(feature = "diagnostic-row-progress")]
            &crate::domain::operator::contract::views::IgnoredProgress,
        )
        .next()
        .is_none(),
        "equal hash words must not erase value kinds"
    );
    Ok(1)
}

fn stored_keys_check_exact_composite_equality_after_a_real_hash_collision() -> Result<Value, Failure> {
    const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Number];
    let colliding_word = 0x517c_c1b7_2722_0a95_u64.rotate_left(5);
    let columns = [
        FactColumn(vec![0, 1, 0]),
        FactColumn(vec![0, colliding_word, 0]),
    ];
    let view = ColumnsView {
        kinds: KINDS,
        columns: &columns,
    };
    let on = [ColumnIndex(0), ColumnIndex(1)];
    let index = prepared_index(&Rows::Stored(view), &on).expect("key");
    assert_eq!(
        index.hashes[0],
        index.hashes[1],
        "fixture must exercise an actual full-hash collision"
    );
    assert_eq!(
        matching_stored(
            view,
            &on,
            &index,
            &[Cell::Number(Integer(0)), Cell::Number(Integer(0))],
            #[cfg(feature = "diagnostic-row-progress")]
            &crate::domain::operator::contract::views::IgnoredProgress,
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(2)]
    );
    Ok(1)
}

fn stored_keys_reuse_empty_indexes_and_validate_columns_before_rows() -> Result<Value, Failure> {
    const KINDS: &[CellKind] = &[CellKind::Number];
    let columns = [FactColumn(vec![7, 7])];
    let empty = [FactColumn(Vec::new())];
    let view = ColumnsView {
        kinds: KINDS,
        columns: &columns,
    };
    let vacant = ColumnsView {
        kinds: KINDS,
        columns: &empty,
    };
    let on = [ColumnIndex(0)];
    let full = prepared_index(&Rows::Stored(view), &on).expect("full");
    let order = prepared_index(&Rows::Stored(vacant), &on).expect("empty");
    assert_eq!(
        full.rows.len(),
        2,
        "new empty payload does not mutate an outstanding full index"
    );
    assert!(
        matching_stored(
            vacant,
            &on,
            &order,
            &[Cell::Number(Integer(7))],
            #[cfg(feature = "diagnostic-row-progress")]
            &crate::domain::operator::contract::views::IgnoredProgress,
        )
        .next()
        .is_none()
    );
    assert_eq!(
        prepared_index(&Rows::Stored(vacant), &[ColumnIndex(1)]).err(),
        Some(RowsError::NoSuchColumn { index: 1 })
    );
    let order = prepared_index(&Rows::Stored(view), &[]).expect("cartesian");
    assert_eq!(
        matching_stored(
            view,
            &[],
            &order,
            &[],
            #[cfg(feature = "diagnostic-row-progress")]
            &crate::domain::operator::contract::views::IgnoredProgress,
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(1)]
    );
    let malformed = [FactColumn(vec![1]), FactColumn(Vec::new())];
    assert_eq!(
        rectangular(ColumnsView {
            kinds: &[CellKind::Number, CellKind::Number],
            columns: &malformed
        }),
        TableShape::Ragged
    );
    assert_eq!(
        rectangular(ColumnsView {
            kinds: KINDS,
            columns: &malformed
        }),
        TableShape::Ragged
    );
    Ok(1)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("stored_keys_preserve_order_duplicates_and_typed_flag_normalization"),
        word: Word::Succeeds,
        input: || stored_keys_preserve_order_duplicates_and_typed_flag_normalization,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("stored_keys_check_exact_composite_equality_after_a_real_hash_collision"),
        word: Word::Succeeds,
        input: || stored_keys_check_exact_composite_equality_after_a_real_hash_collision,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("stored_keys_reuse_empty_indexes_and_validate_columns_before_rows"),
        word: Word::Succeeds,
        input: || stored_keys_reuse_empty_indexes_and_validate_columns_before_rows,
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
