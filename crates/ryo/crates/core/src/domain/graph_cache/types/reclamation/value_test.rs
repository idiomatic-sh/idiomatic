//! Complete preflight, exact payload credit, live handles and metadata-capacity ownership.

use super::value::{GraphAdmission, index_payload, selection_payload};
use crate::domain::stored_graph::types::footprint::value::{arc_bytes, vector};
use crate::domain::stored_indexes::service::{LIMIT, acquire, new};
use crate::domain::stored_indexes::types::FileIndexes;
use crate::domain::stored_selection::types::{Matches, Selector};
use crate::domain::table_model::types::{KeyIndex, SelectionIdentity};
use ryo_language::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

/// Acquire an ordinary index through its production accounting, keeping its live handle if requested.
fn indexed() -> (FileIndexes, Arc<KeyIndex>) {
    let columns = [FactColumn(vec![1, 2, 3, 4]), FactColumn(vec![4, 3, 2, 1])];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    };
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
    cache.graph_enabled = true;
    let handle = acquire(
        &mut cache,
        Relation::Function,
        source,
        &[ColumnIndex(0)],
        &mut KeyIndex::default(),
    )
    .expect("index")
    .shared
    .expect("admitted");
    (cache, handle)
}

#[test]
fn a_live_index_blocks_credit_until_its_consumer_releases_the_handle() {
    let (mut cache, handle) = indexed();
    let payload = index_payload(&cache.entries[0]).expect("actual allocation");
    let before = cache.bytes;
    let capacity = cache.entries.capacity();
    cache.limit = before + payload / 2;
    assert!(!GraphAdmission::new(&mut cache).prepare(payload));
    assert_eq!(cache.bytes, before);
    assert_eq!(cache.entries.len(), 1);
    assert_eq!(cache.graph_counts.pinned_refusals, 1);
    drop(handle);
    assert!(GraphAdmission::new(&mut cache).prepare(payload));
    assert_eq!(cache.bytes, before - payload);
    assert!(cache.entries.is_empty());
    assert_eq!(cache.entries.capacity(), capacity);
    assert_eq!(cache.graph_counts.reclaimed_indexes, 1);
    assert_eq!(cache.graph_counts.reclaimed_bytes, payload as u64);
    assert!(cache.bytes + payload <= cache.limit);
}

#[test]
fn impossible_admission_does_not_mutate_even_dormant_payload_or_capacity() {
    let (mut cache, handle) = indexed();
    drop(handle);
    let before = cache.bytes;
    let capacity = cache.entries.capacity();
    let payload = index_payload(&cache.entries[0]).expect("payload");
    cache.limit = before;
    assert!(!GraphAdmission::new(&mut cache).prepare(payload + 1));
    assert_eq!(
        (cache.bytes, cache.entries.len(), cache.entries.capacity()),
        (before, 1, capacity)
    );
    assert_eq!(cache.graph_counts.reclaimed_bytes, 0);
}

#[test]
fn fifo_reclaims_only_enough_dormant_entries_and_preserves_survivor_order() {
    let (mut cache, handle) = indexed();
    drop(handle);
    let columns = [FactColumn(vec![1, 2, 3, 4]), FactColumn(vec![4, 3, 2, 1])];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    };
    drop(
        acquire(
            &mut cache,
            Relation::Function,
            source,
            &[ColumnIndex(1)],
            &mut KeyIndex::default(),
        )
        .expect("second index"),
    );
    let before = cache.bytes;
    let capacity = cache.entries.capacity();
    let first = index_payload(&cache.entries[0]).expect("payload");
    cache.limit = before;
    assert!(GraphAdmission::new(&mut cache).prepare(first));
    assert_eq!(cache.entries.len(), 1);
    assert_eq!(cache.entries[0].columns, [ColumnIndex(1)]);
    assert_eq!(cache.entries.capacity(), capacity);
    assert_eq!(cache.bytes, before - first);
}

#[test]
fn selected_row_ids_pinned_by_a_graph_or_view_cannot_be_reclaimed() {
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
    let rows = Arc::new(vec![0_u32, 1, 1, 2]);
    cache.selections.push(Matches {
        selector: Arc::new(Selector {
            distinct: false,
            relation: Relation::Function,
            identity: Arc::new(SelectionIdentity::default()),
            columns: vec![ColumnIndex(0)],
        }),
        kinds: &[CellKind::Number],
        population: 3,
        rows: Arc::clone(&rows),
    });
    let payload = selection_payload(&cache.selections[0]).expect("payload");
    assert_eq!(payload, arc_bytes::<Vec<u32>>() + vector(&rows));
    cache.bytes += cache.selections.capacity() * size_of::<Matches>() + payload;
    let before = cache.bytes;
    let capacity = cache.selections.capacity();
    cache.limit = before;
    assert!(!GraphAdmission::new(&mut cache).prepare(payload));
    assert_eq!(cache.bytes, before);
    assert_eq!(cache.selections.len(), 1);
    drop(rows);
    assert!(GraphAdmission::new(&mut cache).prepare(payload));
    assert_eq!(cache.bytes, before - payload);
    assert_eq!(cache.selections.capacity(), capacity);
    assert!(cache.selections.is_empty());
    assert_eq!(cache.graph_counts.reclaimed_selections, 1);
}

#[test]
fn enabling_graph_demand_does_not_reserve_unused_ordinary_capacity() {
    let (mut cache, handle) = indexed();
    drop(handle);
    assert_eq!(
        crate::domain::stored_indexes::service::non_graph_limit(&cache),
        LIMIT
    );
    let bytes = cache.bytes;
    assert!(GraphAdmission::new(&mut cache).prepare(1));
    assert_eq!(cache.bytes, bytes);
    assert_eq!(cache.entries.len(), 1);
}
