use super::ordered_positions;
use crate::domain::operator::contract::views::BorrowedRows;
use crate::domain::table_groups::service::sort_rows;
use ryo_derived::domain::contract::types::{KeyIndex, On};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::service::push_row;

fn table(keys: &[i64]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    for (place, key) in keys.iter().enumerate() {
        push_row(
            &mut table,
            &[
                Value::Number(Integer(*key)),
                Value::Number(Integer(place as i64)),
            ],
        );
    }
    table
}

#[test]
fn borrowed_order_matches_the_existing_group_order() {
    for keys in [
        vec![],
        vec![3],
        vec![3, 3, 2, 2, 1],
        vec![3, 2, 3, 1, 2, 1],
        vec![0, -1, i64::MIN, i64::MAX, 0],
    ] {
        let source = table(&keys);
        let columns = [ColumnIndex(0)];
        let mut legacy = KeyIndex::default();
        sort_rows(&source, On::Columns(&columns), &mut legacy).unwrap();
        let actual = ordered_positions(BorrowedRows::Computed(&source), &columns).unwrap();
        assert_eq!(actual, legacy.sorted, "key sequence {keys:?}");
    }
}

#[test]
fn empty_key_keeps_the_original_row_order() {
    let source = table(&[5, 3, 5, 2]);
    let actual = ordered_positions(BorrowedRows::Computed(&source), &[]).unwrap();
    assert_eq!(
        actual.iter().map(|row| row.0).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
}

use ryo_values::domain::rows::types::Table;
