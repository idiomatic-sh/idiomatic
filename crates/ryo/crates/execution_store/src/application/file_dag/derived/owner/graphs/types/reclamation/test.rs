//! Graph admission reclaims only dormant resources owned by the real file owner.

use super::constants;
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

fn rows<'a>(columns: &'a [FactColumn; 2]) -> Rows<'a> {
    Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns,
    })
}

fn columns() -> [FactColumn; 2] {
    [FactColumn(vec![1, 2, 3, 4]), FactColumn(vec![4, 3, 2, 1])]
}

#[test]
fn a_live_index_blocks_credit_until_its_consumer_releases_the_handle() {
    let columns = columns();
    let rows = rows(&columns);
    let mut fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    let handle = fixture
        .index(Relation::Function, &rows, &[ColumnIndex(0)])
        .expect(constants::OWNER_ACQUIRED_INDEX);
    let before = fixture.snapshot();
    assert_eq!(before.indexes, 1);
    let payload = before.index_payloads[0];
    let capacity = before.owner_entries_capacity;
    let cost = payload;
    fixture.set_limit(before.bytes + payload / 2);
    assert!(!fixture.reclaim(cost));
    let pinned = fixture.snapshot();
    assert_eq!(pinned.bytes, before.bytes);
    assert_eq!(pinned.indexes, 1);
    assert_eq!(pinned.counts.pinned_refusals, 1);
    drop(handle);
    assert!(fixture.reclaim(cost));
    let reclaimed = fixture.snapshot();
    assert_eq!(
        reclaimed.bytes + reclaimed.counts.reclaimed_bytes as usize,
        before.bytes,
        "the owner accounts for the unique physical credit once"
    );
    assert_eq!(reclaimed.indexes, 0);
    assert_eq!(reclaimed.owner_entries_capacity, capacity);
    assert_eq!(reclaimed.counts.reclaimed_indexes, 1);
    assert_eq!(reclaimed.counts.reclaimed_bytes, cost as u64);
    assert_eq!(reclaimed.optional_bytes, 0);
    assert_eq!(reclaimed.retained_view_bytes, 0);
    assert!(reclaimed.bytes + cost <= reclaimed.limit);
}

#[test]
fn impossible_admission_does_not_mutate_even_dormant_payload_or_capacity() {
    let columns = columns();
    let rows = rows(&columns);
    let mut fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    let handle = fixture
        .index(Relation::Function, &rows, &[ColumnIndex(0)])
        .expect(constants::OWNER_ACQUIRED_INDEX);
    drop(handle);
    let before = fixture.snapshot();
    let payload = before.index_payloads[0];
    fixture.set_limit(before.bytes);
    assert!(!fixture.reclaim(payload + 1));
    let after = fixture.snapshot();
    assert_eq!(
        (after.bytes, after.indexes, after.owner_entries_capacity),
        (before.bytes, before.indexes, before.owner_entries_capacity)
    );
    assert_eq!(after.counts.reclaimed_bytes, 0);
}

#[test]
fn fifo_reclaims_only_enough_dormant_entries_and_preserves_survivor_order() {
    let columns = columns();
    let rows = rows(&columns);
    let mut fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    drop(
        fixture
            .index(Relation::Function, &rows, &[ColumnIndex(0)])
            .expect(constants::FIRST_OWNER_ACQUIRED_INDEX),
    );
    drop(
        fixture
            .index(Relation::Function, &rows, &[ColumnIndex(1)])
            .expect(constants::SECOND_OWNER_ACQUIRED_INDEX),
    );
    let before = fixture.snapshot();
    assert_eq!(before.indexes, 2);
    let capacity = before.owner_entries_capacity;
    let first = before.index_payloads[0];
    fixture.set_limit(before.bytes);
    assert!(fixture.reclaim(first));
    let after = fixture.snapshot();
    assert_eq!(after.indexes, 1);
    assert_eq!(after.index_columns[0], [ColumnIndex(1)]);
    assert_eq!(after.owner_entries_capacity, capacity);
    assert!(after.bytes <= before.bytes - first);
}

#[test]
fn owner_reclaims_the_oldest_complete_index_before_newer_indexes() {
    let columns = columns();
    let rows = rows(&columns);
    let mut fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    let first = fixture
        .index(Relation::Function, &rows, &[ColumnIndex(0)])
        .expect(constants::FIRST_OWNER_VIEW);
    let first_storage = Arc::downgrade(first.storage());
    let first_view_bytes = fixture.snapshot().retained_view_bytes;
    assert!(first_view_bytes > 0);
    drop(first);

    let second = fixture
        .index(Relation::Function, &rows, &[ColumnIndex(1)])
        .expect(constants::SECOND_OWNER_VIEW);
    let second_storage = Arc::downgrade(second.storage());
    drop(second);
    let before = fixture.snapshot();
    assert_eq!(before.indexes, 2);
    assert!(before.retained_view_bytes > first_view_bytes);

    fixture.set_limit(before.bytes);
    assert!(fixture.reclaim(first_view_bytes));
    let after = fixture.snapshot();
    assert!(
        first_storage.upgrade().is_none(),
        "oldest owner view releases first"
    );
    assert!(
        second_storage.upgrade().is_some(),
        "newer owner view remains"
    );
    assert_eq!(after.indexes, 1);
    assert_eq!(after.counts.reclaimed_indexes, 1);
    assert_eq!(
        after.retained_view_bytes,
        before.retained_view_bytes - first_view_bytes
    );
}

#[test]
fn enabling_graph_demand_does_not_reserve_unused_ordinary_capacity() {
    let columns = columns();
    let rows = rows(&columns);
    let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    drop(
        fixture
            .index(Relation::Function, &rows, &[ColumnIndex(0)])
            .expect(constants::OWNER_ACQUIRED_INDEX),
    );
    let before = fixture.snapshot();
    assert_eq!(before.limit, LIMIT);
    assert!(fixture.reclaim(1));
    let after = fixture.snapshot();
    assert_eq!(after.bytes, before.bytes);
    assert_eq!(after.indexes, 1);
}
