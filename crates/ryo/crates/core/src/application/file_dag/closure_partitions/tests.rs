use super::PartitionedClosure;
use crate::domain::operator::contract::views::BorrowedRows;
use crate::domain::table_model::service::push_row;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;

fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}

#[test]
fn interleaved_partitions_keep_one_reused_adjacency() {
    let partitions = 2048;
    let mut input = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    for (from, to) in [(1, 2), (2, 3), (1, 4)] {
        for partition in 0..partitions {
            push_row(&mut input, &[number(from), number(to), number(partition)]);
        }
    }
    let mut cursor = PartitionedClosure::prepare(
        BorrowedRows::Computed(&input),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
    )
    .unwrap();
    let mut seen = 0;
    let mut maximum_live = 0;
    let mut maximum_reserved = 0;
    while cursor.advance().unwrap() {
        let part = cursor.current();
        assert_eq!(part.table.count.0, 3);
        assert_eq!(part.table.cells.len(), 6);
        assert_eq!(part.key.len(), 1);
        assert_eq!(part.sources, vec![number(1), number(2)]);
        seen += 1;
        maximum_live = maximum_live.max(cursor.working_cells());
        maximum_reserved = maximum_reserved.max(cursor.working_capacity());
    }
    assert_eq!(seen, partitions as usize);
    assert!(
        maximum_live <= 16,
        "all owned Value payload, including key and sources: {maximum_live}"
    );
    assert!(
        maximum_reserved < 64,
        "retained Value capacity: {maximum_reserved}"
    );
    // Ordering metadata scales with row IDs, not the number of partition graphs.
    assert!(cursor.ordering_bytes() < input.count.0 * 64);
    assert!(!cursor.advance().unwrap());
}

#[test]
fn separate_keys_never_share_edges() {
    let mut input = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    for (from, to, key) in [(1, 2, 5), (9, 10, 7), (2, 3, 5), (10, 11, 7)] {
        push_row(&mut input, &[number(from), number(to), number(key)]);
    }
    let mut cursor = PartitionedClosure::prepare(
        BorrowedRows::Computed(&input),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
    )
    .unwrap();
    let mut observed = Vec::new();
    while cursor.advance().unwrap() {
        observed.push((
            cursor.current().key.clone(),
            cursor.current().table.cells.clone(),
        ));
    }
    assert_eq!(observed.len(), 2);
    assert!(observed.contains(&(
        vec![number(5)],
        vec![number(1), number(2), number(2), number(3)]
    )));
    assert!(observed.contains(&(
        vec![number(7)],
        vec![number(9), number(10), number(10), number(11)]
    )));
}

#[test]
fn advancing_releases_only_the_previous_partition_lease() {
    let mut input = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    for (from, to, key) in [(1, 2, 5), (3, 4, 7)] {
        push_row(&mut input, &[number(from), number(to), number(key)]);
    }
    let mut cursor = PartitionedClosure::prepare(
        BorrowedRows::Computed(&input),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
    )
    .unwrap();
    assert!(cursor.advance().unwrap());
    let first = std::sync::Arc::downgrade(cursor.current_lease.as_ref().unwrap());
    assert!(cursor.advance().unwrap());
    assert!(
        first.upgrade().is_none(),
        "the prior partition was released"
    );
    let current = std::sync::Arc::downgrade(cursor.current_lease.as_ref().unwrap());
    let (part, transferred) = cursor.take_current().unwrap();
    assert_eq!(part.key, vec![number(7)]);
    assert!(cursor.current_lease.is_none());
    drop(cursor);
    assert!(
        current.upgrade().is_some(),
        "the consumer still owns the charge"
    );
    drop(transferred);
    assert!(current.upgrade().is_none());
}

#[test]
fn empty_order_and_wrong_prepared_order_are_refused_precisely() {
    let input = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    let cursor = PartitionedClosure::prepare(
        BorrowedRows::Computed(&input),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
    )
    .unwrap();
    let mut empty = cursor;
    assert!(!empty.advance().unwrap());
    assert!(matches!(
        empty.take_current(),
        Err(crate::domain::operating::errors::OperatingError::NotRun)
    ));
    let wrong_plan = std::sync::Arc::new((*empty.partition_plan).clone());
    assert!(matches!(
        PartitionedClosure::from_prepared(
            BorrowedRows::Computed(&input),
            wrong_plan,
            std::sync::Arc::clone(&empty.closure_plan),
            empty.ordered.clone(),
            std::rc::Rc::clone(&empty.owner),
        ),
        Err(crate::domain::operating::errors::OperatingError::WrongOperation)
    ));
}

use ryo_values::domain::rows::types::Table;
