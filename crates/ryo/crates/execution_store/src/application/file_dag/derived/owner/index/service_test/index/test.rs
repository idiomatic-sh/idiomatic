//! Exact sharing keys, physical partiality, bounded retention and generation-local lifetimes.
use super::super::super::service::LIMIT;
use super::super::super::types::ByteLimit;
use super::fixtures::{acquire, equality, new};
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::{Lacking, LackingKind, Partial};

use ryo_execution_domain::domain::table_model::service::matching_stored;

use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Flag, Integer, Relation, Value as CellValue,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::types::RowIndex;
use ryo_values::domain::stored_facts::types::PartialCause;
use std::borrow::Cow;
use std::sync::Arc;

/// The index the owner is asked for, and around what.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The rows `(2, 20), (1, 10), (2, 21), (2, 22)` on the first column, twice; then key 2.
    Duplicates,
    /// One population under four key orders and relations, then under a wrong schema and a
    /// shorter population, then the shorter one in a fresh owner.
    DistinctKeys,
    /// A population whose relation has a physical partial, as Function and as Identifier.
    Partial,
    /// 1024 rows under a limit that leaves no room past the owner's metadata.
    NoRoom,
    /// The rows `1, 1` in an owner that is dropped, then in a next owner.
    DroppedOwner,
    /// The flags `1, 2, 0` on a column that does not exist, then twice on the first; then the keys
    /// raised and the number 1.
    Flags,
    /// The rows `1, 1, 1` on keys of widths 1 to 64, all held while the owner drops.
    Widths(usize),
    /// One symbol column, reused after 128 derived symbols are interned.
    Symbols(u32),
    /// The rows `(0, 0), (1, c), (0, 0)`, where `(1, c)` hashes as `(0, 0)`, twice; then key
    /// `(0, 0)`.
    Collision,
}

/// What an index asked for twice answered: how the second was constructed and the rows a key
/// matches in it.
#[derive(Debug, PartialEq)]
struct Reused {
    construction: Construction,
    matched: Cow<'static, [RowIndex]>,
}

/// What the asks observed.
#[derive(Debug, PartialEq)]
enum Observed {
    /// How the first was constructed, whether both share storage and the indexes owned.
    Duplicates {
        first: Construction,
        one_storage: bool,
        indexes: usize,
        reused: Reused,
    },
    /// How each of the four keys was constructed, the two refusals, the indexes owned, and how
    /// the shorter population was constructed in a fresh owner.
    DistinctKeys {
        keys: [Construction; 4],
        wrong_schema: Option<OperatingError>,
        shorter: Option<OperatingError>,
        indexes: usize,
        fresh: Construction,
    },
    /// Per relation, how it was constructed and its rows; and the indexes owned.
    Partial {
        local: [(Construction, usize); 2],
        indexes: usize,
    },
    /// How it was constructed, its rows, the indexes owned, whether bytes stayed within the
    /// limit, whether required bytes were charged, whether the backing outlived its handle, and
    /// the required bytes after the generation ends.
    NoRoom {
        construction: Construction,
        rows: usize,
        indexes: usize,
        within_limit: bool,
        required: bool,
        backing_outlived: bool,
        required_after_clear: usize,
    },
    /// Whether the dropped owner's index survived, and how the next was constructed and the
    /// indexes it owns.
    Dropped {
        survived: bool,
        next: Construction,
        indexes: usize,
    },
    /// The refusal of the missing column, the indexes after it, and the rows each key
    /// matches on reuse.
    Flags {
        refused: Option<OperatingError>,
        indexes: usize,
        raised: Cow<'static, [RowIndex]>,
        number: Cow<'static, [RowIndex]>,
    },
    /// In how many widths bytes stayed within the limit and required bytes were charged, and
    /// whether the backing survived the owner and then its last consumer.
    Widths {
        within_limit: usize,
        required: usize,
        survived_owner: bool,
        survived_consumers: bool,
    },
    Reused(Reused),
    /// Whether the two hashes collide, and the reuse.
    Collision {
        collide: bool,
        reused: Reused,
    },
}

type Input = Asked;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("immutable_duplicate_rows_share_one_complete_index_without_cloning_buffers"),
        word: Word::Succeeds,
        input: || Asked::Duplicates,
        expect: Expect::Value(Observed::Duplicates {
            first: Construction::Built,
            one_storage: true,
            indexes: 1,
            reused: Reused {
                construction: Construction::Reused,
                matched: Cow::Borrowed(&[RowIndex(0), RowIndex(2), RowIndex(3)]),
            },
        }),
    },
    Case {
        name: CaseName("relation_order_repeats_schema_and_population_are_distinct_cache_keys"),
        word: Word::Succeeds,
        input: || Asked::DistinctKeys,
        expect: Expect::Value(Observed::DistinctKeys {
            keys: [Construction::Built; 4],
            wrong_schema: Some(OperatingError::NotRead),
            shorter: Some(OperatingError::NotRead),
            indexes: 4,
            fresh: Construction::Built,
        }),
    },
    Case {
        name: CaseName("any_physical_partial_kind_and_inactive_relations_are_ineligible"),
        word: Word::Succeeds,
        input: || Asked::Partial,
        expect: Expect::Value(Observed::Partial {
            local: [(Construction::Built, 1), (Construction::Built, 1)],
            indexes: 0,
        }),
    },
    Case {
        name: CaseName("actual_capacities_and_arc_metadata_remain_bounded_on_refusal"),
        word: Word::Succeeds,
        input: || Asked::NoRoom,
        expect: Expect::Value(Observed::NoRoom {
            construction: Construction::Built,
            rows: 1024,
            indexes: 0,
            within_limit: true,
            required: true,
            backing_outlived: false,
            required_after_clear: 0,
        }),
    },
    Case {
        name: CaseName("cache_drop_releases_generation_and_new_file_id_reuse_starts_empty"),
        word: Word::Succeeds,
        input: || Asked::DroppedOwner,
        expect: Expect::Value(Observed::Dropped {
            survived: false,
            next: Construction::Built,
            indexes: 1,
        }),
    },
    Case {
        name: CaseName("invalid_columns_never_admit_and_normalized_flags_still_match_on_reuse"),
        word: Word::Succeeds,
        input: || Asked::Flags,
        expect: Expect::Value(Observed::Flags {
            refused: Some(OperatingError::Table(RowsError::NoSuchColumn { index: 9 })),
            indexes: 0,
            raised: Cow::Borrowed(&[RowIndex(0), RowIndex(1)]),
            number: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("repeated_admissions_never_evict_live_entries_or_exceed_actual_retention"),
        word: Word::Succeeds,
        input: || Asked::Widths(64),
        expect: Expect::Value(Observed::Widths {
            within_limit: 64,
            required: 64,
            survived_owner: true,
            survived_consumers: false,
        }),
    },
    Case {
        name: CaseName("derived_symbol_growth_does_not_change_a_frozen_stored_key"),
        word: Word::Succeeds,
        input: || Asked::Symbols(128),
        expect: Expect::Value(Observed::Reused(Reused {
            construction: Construction::Reused,
            matched: Cow::Borrowed(&[RowIndex(0)]),
        })),
    },
    Case {
        name: CaseName("a_reused_index_still_resolves_full_composite_hash_collisions_exactly"),
        word: Word::Succeeds,
        input: || Asked::Collision,
        expect: Expect::Value(Observed::Collision {
            collide: true,
            reused: Reused {
                construction: Construction::Reused,
                matched: Cow::Borrowed(&[RowIndex(0), RowIndex(2)]),
            },
        }),
    },
];

fn matched(
    view: ColumnsView<'_>,
    on: &[ColumnIndex],
    index: &ryo_derived::domain::equality::types::KeyIndex,
    key: &[CellValue],
) -> Cow<'static, [RowIndex]> {
    Cow::Owned(
        matching_stored(
            view,
            on,
            index,
            key,
            #[cfg(feature = "diagnostic-row-progress")]
            &ryo_execution_domain::domain::operating::types::IgnoredProgress,
        )
        .copied()
        .collect(),
    )
}

fn numbers<'a>(kinds: &'static [CellKind], columns: &'a [FactColumn]) -> ColumnsView<'a> {
    ColumnsView { kinds, columns }
}

const NUMBER: &[CellKind] = &[CellKind::Number];
const NUMBERS: &[CellKind] = &[CellKind::Number, CellKind::Number];

/// Asks the owner for the index as the input says.
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Asked::Duplicates => {
            let columns = [
                FactColumn(vec![2, 1, 2, 2]),
                FactColumn(vec![20, 10, 21, 22]),
            ];
            let view = numbers(NUMBERS, &columns);
            let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
            let on = [ColumnIndex(0)];
            let first = acquire(&cache, Relation::Function, view, &on)?;
            let second = acquire(&cache, Relation::Function, view, &on)?;
            Observed::Duplicates {
                first: first.construction(),
                one_storage: first.storage_identity() == second.storage_identity(),
                indexes: cache.snapshot().indexes,
                reused: Reused {
                    construction: second.construction(),
                    matched: matched(
                        view,
                        &on,
                        equality(&second),
                        &[CellValue::Number(Integer(2))],
                    ),
                },
            }
        }
        Asked::DistinctKeys => {
            let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 1])];
            let view = numbers(NUMBERS, &columns);
            let cache = new(
                &[Relation::Function, Relation::Identifier],
                &[],
                ByteLimit(LIMIT),
            );
            let mut keys = [Construction::Reused; 4];
            for (construction, (relation, on)) in keys.iter_mut().zip([
                (Relation::Function, [ColumnIndex(0), ColumnIndex(1)]),
                (Relation::Function, [ColumnIndex(1), ColumnIndex(0)]),
                (Relation::Function, [ColumnIndex(0), ColumnIndex(0)]),
                (Relation::Identifier, [ColumnIndex(0), ColumnIndex(1)]),
            ]) {
                *construction = acquire(&cache, relation, view, &on)?.construction();
            }
            let flags = ColumnsView {
                kinds: &[CellKind::Flag, CellKind::Number],
                columns: &columns,
            };
            let wrong_schema = acquire(
                &cache,
                Relation::Function,
                flags,
                &[ColumnIndex(0), ColumnIndex(1)],
            )
            .err();
            let shorter_columns = [FactColumn(vec![1]), FactColumn(vec![2])];
            let shorter = numbers(NUMBERS, &shorter_columns);
            let shorter_refused =
                acquire(&cache, Relation::Function, shorter, &[ColumnIndex(0)]).err();
            let fresh = new(&[Relation::Function], &[], ByteLimit(LIMIT));
            Observed::DistinctKeys {
                keys,
                wrong_schema,
                shorter: shorter_refused,
                indexes: cache.snapshot().indexes,
                fresh: acquire(&fresh, Relation::Function, shorter, &[ColumnIndex(0)])?
                    .construction(),
            }
        }
        Asked::Partial => {
            let partials = [Partial {
                relation: Relation::Function,
                cause: PartialCause("some function kinds absent".into()),
                lacking: Lacking::Kind(LackingKind("unknown".into())),
                at: ryo_execution_domain::domain::coverage::types::PartialPlace::Repository,
            }];
            let cache = new(&[Relation::Function], &partials, ByteLimit(LIMIT));
            let columns = [FactColumn(vec![1])];
            let view = numbers(NUMBER, &columns);
            let mut local = [(Construction::Reused, 0); 2];
            for (answer, relation) in local
                .iter_mut()
                .zip([Relation::Function, Relation::Identifier])
            {
                let result = acquire(&cache, relation, view, &[ColumnIndex(0)])?;
                *answer = (result.construction(), equality(&result).rows.len());
            }
            Observed::Partial {
                local,
                indexes: cache.snapshot().indexes,
            }
        }
        Asked::NoRoom => {
            let columns = [FactColumn((0..1024).collect())];
            let view = numbers(NUMBER, &columns);
            let mut cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
            let metadata = cache.snapshot().bytes;
            cache.set_limit(metadata);
            let result = acquire(&cache, Relation::Function, view, &[ColumnIndex(0)])?;
            let construction = result.construction();
            let rows = equality(&result).rows.len();
            let during = cache.snapshot();
            let backing = Arc::downgrade(result.storage());
            drop(result);
            let backing_outlived = backing.upgrade().is_some();
            // Source admission owns a separate publication record until this generation ends.
            cache.clear();
            Observed::NoRoom {
                construction,
                rows,
                indexes: during.indexes,
                within_limit: during.bytes <= during.limit,
                required: during.required_bytes > 0,
                backing_outlived,
                required_after_clear: cache.snapshot().required_bytes,
            }
        }
        Asked::DroppedOwner => {
            let columns = [FactColumn(vec![1, 1])];
            let view = numbers(NUMBER, &columns);
            let weak = {
                let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
                let acquired = acquire(&cache, Relation::Function, view, &[ColumnIndex(0)])?;
                Arc::downgrade(acquired.storage())
            };
            let survived = weak.upgrade().is_some();
            let next = new(&[Relation::Function], &[], ByteLimit(LIMIT));
            let construction =
                acquire(&next, Relation::Function, view, &[ColumnIndex(0)])?.construction();
            Observed::Dropped {
                survived,
                next: construction,
                indexes: next.snapshot().indexes,
            }
        }
        Asked::Flags => {
            let columns = [FactColumn(vec![1, 2, 0])];
            let view = ColumnsView {
                kinds: &[CellKind::Flag],
                columns: &columns,
            };
            let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
            let refused = acquire(&cache, Relation::Function, view, &[ColumnIndex(9)]).err();
            let indexes = cache.snapshot().indexes;
            drop(acquire(
                &cache,
                Relation::Function,
                view,
                &[ColumnIndex(0)],
            )?);
            let reused = acquire(&cache, Relation::Function, view, &[ColumnIndex(0)])?;
            Observed::Flags {
                refused,
                indexes,
                raised: matched(
                    view,
                    &[ColumnIndex(0)],
                    equality(&reused),
                    &[CellValue::Flag(Flag::Raised)],
                ),
                number: matched(
                    view,
                    &[ColumnIndex(0)],
                    equality(&reused),
                    &[CellValue::Number(Integer(1))],
                ),
            }
        }
        Asked::Widths(widest) => {
            let columns = [FactColumn(vec![1, 1, 1])];
            let view = numbers(NUMBER, &columns);
            let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
            let mut handles = Vec::new();
            let (mut within_limit, mut required) = (0, 0);
            for width in 1..=widest {
                let key = vec![ColumnIndex(0); width];
                handles.push(acquire(&cache, Relation::Function, view, &key)?);
                let state = cache.snapshot();
                within_limit += usize::from(state.bytes <= state.limit);
                required += usize::from(state.required_bytes > 0);
            }
            let weak = Arc::downgrade(handles[0].storage());
            drop(cache);
            let survived_owner = weak.upgrade().is_some();
            drop(handles);
            Observed::Widths {
                within_limit,
                required,
                survived_owner,
                survived_consumers: weak.upgrade().is_some(),
            }
        }
        Asked::Symbols(interned) => {
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
            drop(acquire(
                &cache,
                Relation::Function,
                view,
                &[ColumnIndex(0)],
            )?);
            for index in 0..interned {
                let _derived = intern_derived(
                    &mut symbols,
                    ByteSlice(b""),
                    NameText(&format!("derived_{index}")),
                );
            }
            let reused = acquire(&cache, Relation::Function, view, &[ColumnIndex(0)])?;
            Observed::Reused(Reused {
                construction: reused.construction(),
                matched: matched(
                    view,
                    &[ColumnIndex(0)],
                    equality(&reused),
                    &[CellValue::Symbol(original)],
                ),
            })
        }
        Asked::Collision => {
            // One FxHash step sees only the running hash plus the word, so the key (1, -mix(0, 1)) hashes as (0, 0).
            let collision = ryo_derived::domain::key::service::mix(
                ryo_derived::domain::key::types::KeyCellsHash(0),
                ryo_derived::domain::key::types::HashWord(1),
            )
            .0
            .wrapping_neg();
            let columns = [FactColumn(vec![0, 1, 0]), FactColumn(vec![0, collision, 0])];
            let view = numbers(NUMBERS, &columns);
            let on = [ColumnIndex(0), ColumnIndex(1)];
            let cache = new(&[Relation::Function], &[], ByteLimit(LIMIT));
            drop(acquire(&cache, Relation::Function, view, &on)?);
            let reused = acquire(&cache, Relation::Function, view, &on)?;
            let index = equality(&reused);
            Observed::Collision {
                collide: index.hashes.first() == index.hashes.get(1),
                reused: Reused {
                    construction: reused.construction(),
                    matched: matched(
                        view,
                        &on,
                        index,
                        &[CellValue::Number(Integer(0)), CellValue::Number(Integer(0))],
                    ),
                },
            }
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
