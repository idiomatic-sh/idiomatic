//! Exact sharing keys, physical partiality, bounded retention and generation-local lifetimes.
use super::{LIMIT, acquire_fixture as acquire, index_bytes, new};
use crate::domain::coverage::types::{Lacking, LackingKind, Partial};
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::service::matching_stored;
use crate::domain::table_model::types::KeyIndex;
use ryo_language::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Flag, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::stored_facts::types::PartialCause;
use std::sync::Arc;

#[test]
fn immutable_duplicate_rows_share_one_complete_index_without_cloning_buffers() {
    let columns = [
        FactColumn(vec![2, 1, 2, 2]),
        FactColumn(vec![20, 10, 21, 22]),
    ];
    let view = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Number],
        columns: &columns,
    };
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
    let mut local = KeyIndex::default();
    let on = [ColumnIndex(0)];
    let first = acquire(&mut cache, Relation::Function, view, &on, &mut local).expect("index");
    assert!(first.built && first.eligible);
    let second = acquire(&mut cache, Relation::Function, view, &on, &mut local).expect("reuse");
    assert!(!second.built && second.eligible);
    let first = first.shared.expect("shared first");
    let second = second.shared.expect("shared second");
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!((cache.builds, cache.reuses, cache.entries.len()), (1, 1, 1));
    assert_eq!(
        matching_stored(view, &on, &second, &[Value::Number(Integer(2))])
            .copied()
            .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(2), RowIndex(3)]
    );
    assert!(
        local.rows.is_empty(),
        "shared index vectors never copy into invocation scratch"
    );
}

#[test]
fn relation_order_repeats_schema_and_population_are_distinct_cache_keys() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 1])];
    let view = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Number],
        columns: &columns,
    };
    let mut cache = new(&[Relation::Function, Relation::Identifier], &[], LIMIT).expect("cache");
    let mut local = KeyIndex::default();
    for (relation, on) in [
        (Relation::Function, vec![ColumnIndex(0), ColumnIndex(1)]),
        (Relation::Function, vec![ColumnIndex(1), ColumnIndex(0)]),
        (Relation::Function, vec![ColumnIndex(0), ColumnIndex(0)]),
        (Relation::Identifier, vec![ColumnIndex(0), ColumnIndex(1)]),
    ] {
        assert!(
            acquire(&mut cache, relation, view, &on, &mut local)
                .expect("new key")
                .built
        );
    }
    let flags = ColumnsView {
        kinds: &[CellKind::Flag, CellKind::Number],
        columns: &columns,
    };
    assert!(
        acquire(
            &mut cache,
            Relation::Function,
            flags,
            &[ColumnIndex(0), ColumnIndex(1)],
            &mut local
        )
        .expect("typed key")
        .built
    );
    let shorter = [FactColumn(vec![1]), FactColumn(vec![2])];
    let shorter = ColumnsView {
        kinds: view.kinds,
        columns: &shorter,
    };
    assert!(
        acquire(
            &mut cache,
            Relation::Function,
            shorter,
            &[ColumnIndex(0), ColumnIndex(1)],
            &mut local
        )
        .expect("population key")
        .built
    );
    assert_eq!(cache.entries.len(), 6);
}

#[test]
fn any_physical_partial_kind_and_inactive_relations_are_ineligible() {
    let partials = [Partial {
        relation: Relation::Function,
        cause: PartialCause("some function kinds absent".into()),
        lacking: Lacking::Kind(LackingKind("unknown".into())),
        at: None,
    }];
    let mut cache = new(&[Relation::Function], &partials, LIMIT).expect("cache");
    let columns = [FactColumn(vec![1])];
    let view = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    for relation in [Relation::Function, Relation::Identifier] {
        let result = acquire(
            &mut cache,
            relation,
            view,
            &[ColumnIndex(0)],
            &mut KeyIndex::default(),
        )
        .expect("local fallback");
        assert!(result.shared.is_none() && result.built && !result.eligible);
    }
    assert!(cache.entries.is_empty());
}

#[test]
fn actual_capacities_and_arc_metadata_remain_bounded_on_refusal() {
    let columns = [FactColumn((0..1024).collect())];
    let view = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    let mut cache = new(&[Relation::Function], &[], 512).expect("metadata fits");
    let initial = cache.bytes;
    let mut local = KeyIndex::default();
    let result = acquire(
        &mut cache,
        Relation::Function,
        view,
        &[ColumnIndex(0)],
        &mut local,
    )
    .expect("local index");
    assert!(result.shared.is_none() && result.built && result.eligible);
    assert_eq!(cache.bytes, initial);
    assert!(cache.bytes <= cache.limit && cache.entries.is_empty());
    assert_eq!(cache.refusals, 1);
    assert_eq!(local.rows.len(), 1024);
    assert!(index_bytes(&local).expect("all capacities counted") > cache.limit);
    assert!(new(&[Relation::Function], &[], 0).is_none());
    local.sorted.reserve_exact(4096);
    let expanded = index_bytes(&local).expect("sorting capacity also counted");
    assert!(expanded >= 4096 * size_of::<RowIndex>());
}

#[test]
fn cache_drop_releases_generation_and_new_file_id_reuse_starts_empty() {
    let columns = [FactColumn(vec![1, 1])];
    let view = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    let weak = {
        let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
        let acquired = acquire(
            &mut cache,
            Relation::Function,
            view,
            &[ColumnIndex(0)],
            &mut KeyIndex::default(),
        )
        .expect("index");
        Arc::downgrade(&acquired.shared.expect("shared"))
    };
    assert!(
        weak.upgrade().is_none(),
        "no generation survives its owner or invocation handles"
    );
    let mut next = new(&[Relation::Function], &[], LIMIT).expect("next generation");
    assert!(
        acquire(
            &mut next,
            Relation::Function,
            view,
            &[ColumnIndex(0)],
            &mut KeyIndex::default()
        )
        .expect("fresh")
        .built
    );
    assert_eq!(next.reuses, 0);
}

#[test]
fn invalid_columns_never_admit_and_normalized_flags_still_match_on_reuse() {
    let columns = [FactColumn(vec![1, 2, 0])];
    let view = ColumnsView {
        kinds: &[CellKind::Flag],
        columns: &columns,
    };
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
    let mut local = KeyIndex::default();
    let _invalid_key = acquire(
        &mut cache,
        Relation::Function,
        view,
        &[ColumnIndex(9)],
        &mut local,
    )
    .expect_err("invalid column must be refused before admission");
    assert!(cache.entries.is_empty());
    drop(
        acquire(
            &mut cache,
            Relation::Function,
            view,
            &[ColumnIndex(0)],
            &mut local,
        )
        .expect("build"),
    );
    let reused = acquire(
        &mut cache,
        Relation::Function,
        view,
        &[ColumnIndex(0)],
        &mut local,
    )
    .expect("reuse")
    .shared
    .expect("index");
    assert_eq!(
        matching_stored(
            view,
            &[ColumnIndex(0)],
            &reused,
            &[Value::Flag(Flag::Raised)]
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(1)]
    );
    assert!(
        matching_stored(
            view,
            &[ColumnIndex(0)],
            &reused,
            &[Value::Number(Integer(1))]
        )
        .next()
        .is_none()
    );
}

#[test]
fn repeated_admissions_never_evict_live_entries_or_exceed_actual_retention() {
    let columns = [FactColumn(vec![1, 1, 1])];
    let view = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    let mut cache = new(&[Relation::Function], &[], 16 * 1024).expect("cache");
    let mut local = KeyIndex::default();
    let mut first = None;
    let mut admitted = 0;
    for width in 1..128 {
        let key = vec![ColumnIndex(0); width];
        let acquired =
            acquire(&mut cache, Relation::Function, view, &key, &mut local).expect("index");
        if let Some(index) = acquired.shared {
            admitted += 1;
            if first.is_none() {
                first = Some(Arc::downgrade(&index));
            }
        }
        assert!(cache.bytes <= cache.limit);
        assert_eq!(cache.entries.len(), admitted);
    }
    assert!(cache.refusals > 0);
    assert!(
        first
            .as_ref()
            .expect("at least one admission")
            .upgrade()
            .is_some()
    );
    drop(cache);
    assert!(first.expect("weak handle").upgrade().is_none());
}

#[test]
fn derived_symbol_growth_does_not_change_a_frozen_stored_key() {
    use ryo_language::domain::symbols::{
        service::intern_derived,
        types::{ByteSlice, Symbols},
    };
    use ryo_language::domain::terms::types::NameText;
    let mut symbols = Symbols::default();
    let original = intern_derived(&mut symbols, ByteSlice(b""), NameText("original"));
    let columns = [FactColumn(vec![u64::from(original.0)])];
    let view = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
    let mut local = KeyIndex::default();
    drop(
        acquire(
            &mut cache,
            Relation::Function,
            view,
            &[ColumnIndex(0)],
            &mut local,
        )
        .expect("index"),
    );
    for index in 0_u32..128 {
        let _derived = intern_derived(
            &mut symbols,
            ByteSlice(b""),
            NameText(&format!("derived_{index}")),
        );
    }
    let reused = acquire(
        &mut cache,
        Relation::Function,
        view,
        &[ColumnIndex(0)],
        &mut local,
    )
    .expect("reuse");
    assert!(!reused.built);
    assert_eq!(
        matching_stored(
            view,
            &[ColumnIndex(0)],
            &reused.shared.expect("index"),
            &[Value::Symbol(original)]
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0)]
    );
}

#[test]
fn a_reused_index_still_resolves_full_composite_hash_collisions_exactly() {
    let collision = 0x517c_c1b7_2722_0a95_u64.rotate_left(5);
    let columns = [FactColumn(vec![0, 1, 0]), FactColumn(vec![0, collision, 0])];
    let view = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Number],
        columns: &columns,
    };
    let on = [ColumnIndex(0), ColumnIndex(1)];
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
    let mut local = KeyIndex::default();
    drop(acquire(&mut cache, Relation::Function, view, &on, &mut local).expect("initial index"));
    let reused =
        acquire(&mut cache, Relation::Function, view, &on, &mut local).expect("reused index");
    assert!(!reused.built);
    let index = reused.shared.expect("shared");
    assert_eq!(
        index.hashes.first(),
        index.hashes.get(1),
        "actual full-hash collision"
    );
    assert_eq!(
        matching_stored(
            view,
            &on,
            &index,
            &[Value::Number(Integer(0)), Value::Number(Integer(0))]
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(2)]
    );
}
