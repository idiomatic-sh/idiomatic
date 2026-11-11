//! Exact sharing keys, physical partiality, bounded retention and generation-local lifetimes.
use super::super::super::service::LIMIT;
use super::super::super::types::ByteLimit;
use super::fixtures::{acquire, equality, new};
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::{Lacking, LackingKind, Partial};

use ryo_execution_domain::domain::table_model::service::matching_stored;

use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Flag, Integer, Relation, Value as CellValue,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::stored_facts::types::PartialCause;
use std::sync::Arc;

fn immutable_duplicate_rows_share_one_complete_index_without_cloning_buffers() {
    let columns = [
        FactColumn(vec![2, 1, 2, 2]),
        FactColumn(vec![20, 10, 21, 22]),
    ];
    let view = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Number],
        columns: &columns,
    };
    let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
    let on = [ColumnIndex(0)];
    let first = acquire(&cache, Relation::Function, view, &on).expect("index");
    assert_eq!(first.construction(), Construction::Built);
    let second = acquire(&cache, Relation::Function, view, &on).expect("reuse");
    assert_eq!(second.construction(), Construction::Reused);
    assert_eq!(first.storage_identity(), second.storage_identity());
    assert_eq!(cache.snapshot().indexes, 1);
    assert_eq!(
        matching_stored(
            view,
            &on,
            equality(&second),
            &[CellValue::Number(Integer(2))],
            #[cfg(feature = "diagnostic-row-progress")]
            &ryo_execution_domain::domain::operating::types::IgnoredProgress,
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(2), RowIndex(3)]
    );
}

fn relation_order_repeats_schema_and_population_are_distinct_cache_keys() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 1])];
    let view = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Number],
        columns: &columns,
    };
    let cache = new(
        &[Relation::Function, Relation::Identifier],
        &[],
        ByteLimit(LIMIT),
    );
    for (relation, on) in [
        (Relation::Function, vec![ColumnIndex(0), ColumnIndex(1)]),
        (Relation::Function, vec![ColumnIndex(1), ColumnIndex(0)]),
        (Relation::Function, vec![ColumnIndex(0), ColumnIndex(0)]),
        (Relation::Identifier, vec![ColumnIndex(0), ColumnIndex(1)]),
    ] {
        assert!(
            acquire(&cache, relation, view, &on)
                .expect("new key")
                .construction()
                == Construction::Built
        );
    }
    let flags = ColumnsView {
        kinds: &[CellKind::Flag, CellKind::Number],
        columns: &columns,
    };
    assert!(
        acquire(
            &cache,
            Relation::Function,
            flags,
            &[ColumnIndex(0), ColumnIndex(1)]
        )
        .is_err()
    );
    let shorter_columns = [FactColumn(vec![1]), FactColumn(vec![2])];
    let shorter = ColumnsView {
        kinds: view.kinds,
        columns: &shorter_columns,
    };
    assert!(acquire(&cache, Relation::Function, shorter, &[ColumnIndex(0)]).is_err());
    assert_eq!(cache.snapshot().indexes, 4);
    let fresh = new(&[Relation::Function], &[], ByteLimit(LIMIT));
    assert!(
        acquire(&fresh, Relation::Function, shorter, &[ColumnIndex(0)])
            .expect("new generation")
            .construction()
            == Construction::Built
    );
}

fn any_physical_partial_kind_and_inactive_relations_are_ineligible() {
    let partials = [Partial {
        relation: Relation::Function,
        cause: PartialCause("some function kinds absent".into()),
        lacking: Lacking::Kind(LackingKind("unknown".into())),
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::Repository,
    }];
    let cache = new(&[Relation::Function], &partials, ByteLimit(LIMIT));
    let columns = [FactColumn(vec![1])];
    let view = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    for relation in [Relation::Function, Relation::Identifier] {
        let result = acquire(&cache, relation, view, &[ColumnIndex(0)]).expect("local fallback");
        assert_eq!(result.construction(), Construction::Built);
        assert_eq!(equality(&result).rows.len(), 1);
    }
    assert_eq!(cache.snapshot().indexes, 0);
}

fn actual_capacities_and_arc_metadata_remain_bounded_on_refusal() {
    let columns = [FactColumn((0..1024).collect())];
    let view = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    let mut cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
    let metadata = cache.snapshot().bytes;
    cache.set_limit(metadata);
    let result =
        acquire(&cache, Relation::Function, view, &[ColumnIndex(0)]).expect("required index");
    assert_eq!(result.construction(), Construction::Built);
    assert_eq!(equality(&result).rows.len(), 1024);
    let during = cache.snapshot();
    assert_eq!(during.indexes, 0);
    assert!(during.bytes <= during.limit);
    assert!(during.required_bytes > 0);
    let backing = Arc::downgrade(result.storage());
    drop(result);
    assert!(
        backing.upgrade().is_none(),
        "the refused optional index has no owner after its invocation"
    );
    // Source admission owns a separate publication record until this generation ends.
    cache.clear();
    assert_eq!(cache.snapshot().required_bytes, 0);
}

fn cache_drop_releases_generation_and_new_file_id_reuse_starts_empty() {
    let columns = [FactColumn(vec![1, 1])];
    let view = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    let weak = {
        let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
        let acquired = acquire(&cache, Relation::Function, view, &[ColumnIndex(0)]).expect("index");
        Arc::downgrade(acquired.storage())
    };
    assert!(
        weak.upgrade().is_none(),
        "no generation survives its owner or invocation handles"
    );
    let next = new(&[Relation::Function], &[], ByteLimit(LIMIT));
    assert!(
        acquire(&next, Relation::Function, view, &[ColumnIndex(0)])
            .expect("fresh")
            .construction()
            == Construction::Built
    );
    assert_eq!(next.snapshot().indexes, 1);
}

fn invalid_columns_never_admit_and_normalized_flags_still_match_on_reuse() {
    let columns = [FactColumn(vec![1, 2, 0])];
    let view = ColumnsView {
        kinds: &[CellKind::Flag],
        columns: &columns,
    };
    let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
    let _invalid_key = acquire(&cache, Relation::Function, view, &[ColumnIndex(9)])
        .expect_err("invalid column must be refused before admission");
    assert_eq!(cache.snapshot().indexes, 0);
    drop(acquire(&cache, Relation::Function, view, &[ColumnIndex(0)]).expect("build"));
    let reused = acquire(&cache, Relation::Function, view, &[ColumnIndex(0)]).expect("reuse");
    assert_eq!(
        matching_stored(
            view,
            &[ColumnIndex(0)],
            equality(&reused),
            &[CellValue::Flag(Flag::Raised)],
            #[cfg(feature = "diagnostic-row-progress")]
            &ryo_execution_domain::domain::operating::types::IgnoredProgress,
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(1)]
    );
    assert!(
        matching_stored(
            view,
            &[ColumnIndex(0)],
            equality(&reused),
            &[CellValue::Number(Integer(1))],
            #[cfg(feature = "diagnostic-row-progress")]
            &ryo_execution_domain::domain::operating::types::IgnoredProgress,
        )
        .next()
        .is_none()
    );
}

fn repeated_admissions_never_evict_live_entries_or_exceed_actual_retention() {
    let columns = [FactColumn(vec![1, 1, 1])];
    let view = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
    let mut handles = Vec::new();
    for width in 1..=64 {
        let key = vec![ColumnIndex(0); width];
        handles.push(acquire(&cache, Relation::Function, view, &key).expect("required result"));
        let state = cache.snapshot();
        assert!(state.bytes <= state.limit);
        assert!(state.required_bytes > 0);
    }
    let weak = Arc::downgrade(handles[0].storage());
    drop(cache);
    assert!(weak.upgrade().is_some(), "active consumers own the backing");
    drop(handles);
    assert!(
        weak.upgrade().is_none(),
        "last required consumer releases it"
    );
}

fn derived_symbol_growth_does_not_change_a_frozen_stored_key() {
    use ryo_language::domain::symbols::{
        service::intern_derived,
        types::{ByteSlice, Symbols},
    };
    use ryo_relations::domain::terms::types::NameText;
    let mut symbols = Symbols::default();
    let original = intern_derived(&mut symbols, ByteSlice(b""), NameText("original"));
    let columns = [FactColumn(vec![u64::from(original.0)])];
    let view = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
    drop(acquire(&cache, Relation::Function, view, &[ColumnIndex(0)]).expect("index"));
    for index in 0_u32..128 {
        let _derived = intern_derived(
            &mut symbols,
            ByteSlice(b""),
            NameText(&format!("derived_{index}")),
        );
    }
    let reused = acquire(&cache, Relation::Function, view, &[ColumnIndex(0)]).expect("reuse");
    assert_eq!(reused.construction(), Construction::Reused);
    assert_eq!(
        matching_stored(
            view,
            &[ColumnIndex(0)],
            equality(&reused),
            &[CellValue::Symbol(original)],
            #[cfg(feature = "diagnostic-row-progress")]
            &ryo_execution_domain::domain::operating::types::IgnoredProgress,
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0)]
    );
}

fn a_reused_index_still_resolves_full_composite_hash_collisions_exactly() {
    // One FxHash step sees only the running hash plus the word, so the key (1, -mix(0, 1)) hashes as (0, 0).
    let collision = ryo_derived::domain::key::service::mix(
        ryo_derived::domain::key::types::KeyCellsHash(0),
        ryo_derived::domain::key::types::HashWord(1),
    )
    .0
    .wrapping_neg();
    let columns = [FactColumn(vec![0, 1, 0]), FactColumn(vec![0, collision, 0])];
    let view = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Number],
        columns: &columns,
    };
    let on = [ColumnIndex(0), ColumnIndex(1)];
    let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
    drop(acquire(&cache, Relation::Function, view, &on).expect("initial index"));
    let reused = acquire(&cache, Relation::Function, view, &on).expect("reused index");
    assert_eq!(reused.construction(), Construction::Reused);
    let index = equality(&reused);
    assert_eq!(
        index.hashes.first(),
        index.hashes.get(1),
        "actual full-hash collision"
    );
    assert_eq!(
        matching_stored(
            view,
            &on,
            index,
            &[CellValue::Number(Integer(0)), CellValue::Number(Integer(0))],
            #[cfg(feature = "diagnostic-row-progress")]
            &ryo_execution_domain::domain::operating::types::IgnoredProgress,
        )
        .copied()
        .collect::<Vec<_>>(),
        [RowIndex(0), RowIndex(2)]
    );
}

use ryo_values::domain::rows::types::RowIndex;

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ImmutableDuplicateRowsShareOneCompleteIndexWithoutCloningBuffers,
    RelationOrderRepeatsSchemaAndPopulationAreDistinctCacheKeys,
    AnyPhysicalPartialKindAndInactiveRelationsAreIneligible,
    ActualCapacitiesAndArcMetadataRemainBoundedOnRefusal,
    CacheDropReleasesGenerationAndNewFileIdReuseStartsEmpty,
    InvalidColumnsNeverAdmitAndNormalizedFlagsStillMatchOnReuse,
    RepeatedAdmissionsNeverEvictLiveEntriesOrExceedActualRetention,
    DerivedSymbolGrowthDoesNotChangeAFrozenStoredKey,
    AReusedIndexStillResolvesFullCompositeHashCollisionsExactly,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("immutable_duplicate_rows_share_one_complete_index_without_cloning_buffers"),
        word: Word::Succeeds,
        input: || Scenario::ImmutableDuplicateRowsShareOneCompleteIndexWithoutCloningBuffers,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("relation_order_repeats_schema_and_population_are_distinct_cache_keys"),
        word: Word::Succeeds,
        input: || Scenario::RelationOrderRepeatsSchemaAndPopulationAreDistinctCacheKeys,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("any_physical_partial_kind_and_inactive_relations_are_ineligible"),
        word: Word::Succeeds,
        input: || Scenario::AnyPhysicalPartialKindAndInactiveRelationsAreIneligible,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("actual_capacities_and_arc_metadata_remain_bounded_on_refusal"),
        word: Word::Succeeds,
        input: || Scenario::ActualCapacitiesAndArcMetadataRemainBoundedOnRefusal,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("cache_drop_releases_generation_and_new_file_id_reuse_starts_empty"),
        word: Word::Succeeds,
        input: || Scenario::CacheDropReleasesGenerationAndNewFileIdReuseStartsEmpty,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalid_columns_never_admit_and_normalized_flags_still_match_on_reuse"),
        word: Word::Succeeds,
        input: || Scenario::InvalidColumnsNeverAdmitAndNormalizedFlagsStillMatchOnReuse,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("repeated_admissions_never_evict_live_entries_or_exceed_actual_retention"),
        word: Word::Succeeds,
        input: || Scenario::RepeatedAdmissionsNeverEvictLiveEntriesOrExceedActualRetention,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("derived_symbol_growth_does_not_change_a_frozen_stored_key"),
        word: Word::Succeeds,
        input: || Scenario::DerivedSymbolGrowthDoesNotChangeAFrozenStoredKey,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_reused_index_still_resolves_full_composite_hash_collisions_exactly"),
        word: Word::Succeeds,
        input: || Scenario::AReusedIndexStillResolvesFullCompositeHashCollisionsExactly,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ImmutableDuplicateRowsShareOneCompleteIndexWithoutCloningBuffers => {
            immutable_duplicate_rows_share_one_complete_index_without_cloning_buffers();
            Ok(())
        }
        Scenario::RelationOrderRepeatsSchemaAndPopulationAreDistinctCacheKeys => {
            relation_order_repeats_schema_and_population_are_distinct_cache_keys();
            Ok(())
        }
        Scenario::AnyPhysicalPartialKindAndInactiveRelationsAreIneligible => {
            any_physical_partial_kind_and_inactive_relations_are_ineligible();
            Ok(())
        }
        Scenario::ActualCapacitiesAndArcMetadataRemainBoundedOnRefusal => {
            actual_capacities_and_arc_metadata_remain_bounded_on_refusal();
            Ok(())
        }
        Scenario::CacheDropReleasesGenerationAndNewFileIdReuseStartsEmpty => {
            cache_drop_releases_generation_and_new_file_id_reuse_starts_empty();
            Ok(())
        }
        Scenario::InvalidColumnsNeverAdmitAndNormalizedFlagsStillMatchOnReuse => {
            invalid_columns_never_admit_and_normalized_flags_still_match_on_reuse();
            Ok(())
        }
        Scenario::RepeatedAdmissionsNeverEvictLiveEntriesOrExceedActualRetention => {
            repeated_admissions_never_evict_live_entries_or_exceed_actual_retention();
            Ok(())
        }
        Scenario::DerivedSymbolGrowthDoesNotChangeAFrozenStoredKey => {
            derived_symbol_growth_does_not_change_a_frozen_stored_key();
            Ok(())
        }
        Scenario::AReusedIndexStillResolvesFullCompositeHashCollisionsExactly => {
            a_reused_index_still_resolves_full_composite_hash_collisions_exactly();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
