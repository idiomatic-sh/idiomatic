//! Graph admission reclaims only dormant resources owned by the real file owner.
use crate::cases::{Case, CaseName, Expect, Word, run};

use super::super::super::constants;
use super::fixtures::{columns, rows};
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

/// The owner's indexes over the rows `(1, 4), (2, 3), (3, 2), (4, 1)` when a graph asks for room.
#[derive(Clone, Copy, Debug)]
enum Indexes {
    /// One index whose consumer still holds it; the limit leaves half its payload short.
    Held,
    /// One dormant index; the room asked is one byte more than it can free.
    DormantShort,
    /// Two dormant indexes, on the first and the second column; the room asked is the first's.
    TwoDormant,
    /// Two dormant indexes as owner views; the room asked is the first's view bytes.
    TwoDormantViews,
    /// One dormant index under the unchanged limit; one byte is asked.
    UnderTheLimit,
}

/// What a held index's reclaim observed, before and after its consumer released it.
#[derive(Debug, PartialEq)]
struct Held {
    indexes: usize,
    granted_while_held: bool,
    bytes_kept_while_held: bool,
    indexes_while_held: usize,
    pinned_refusals: u64,
    granted_after_release: bool,
    credited_once: bool,
    indexes_after: usize,
    capacity_kept: bool,
    reclaimed_indexes: u64,
    reclaimed_the_cost: bool,
    optional_bytes: usize,
    retained_view_bytes: usize,
    cost_fits: bool,
}

/// What a FIFO reclaim of two dormant indexes observed.
#[derive(Debug, PartialEq)]
struct Fifo {
    indexes: usize,
    granted: bool,
    indexes_after: usize,
    survivor: [ColumnIndex; 1],
    capacity_kept: bool,
    freed_the_first: bool,
}

/// What reclaiming the oldest of two owner views observed.
#[derive(Debug, PartialEq)]
struct Oldest {
    first_view_bytes: bool,
    indexes: usize,
    second_adds_view_bytes: bool,
    granted: bool,
    oldest_alive: bool,
    newer_alive: bool,
    indexes_after: usize,
    reclaimed_indexes: u64,
    freed_only_the_first: bool,
}

/// What the room request observed.
#[derive(Debug, PartialEq)]
enum Observed {
    Held(Held),
    /// Whether room was granted, whether bytes, indexes and capacity are unchanged, and the
    /// bytes reclaimed.
    Refused {
        granted: bool,
        unchanged: bool,
        reclaimed_bytes: u64,
    },
    Fifo(Fifo),
    Oldest(Oldest),
    /// The limit, whether room was granted, whether the bytes are unchanged, and the indexes.
    Fits {
        limit: usize,
        granted: bool,
        bytes_unchanged: bool,
        indexes: usize,
    },
}

type Input = Indexes;
type Value = Observed;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_live_index_blocks_credit_until_its_consumer_releases_the_handle"),
        word: Word::Succeeds,
        input: || Indexes::Held,
        expect: Expect::Value(Observed::Held(Held {
            indexes: 1,
            granted_while_held: false,
            bytes_kept_while_held: true,
            indexes_while_held: 1,
            pinned_refusals: 1,
            granted_after_release: true,
            credited_once: true,
            indexes_after: 0,
            capacity_kept: true,
            reclaimed_indexes: 1,
            reclaimed_the_cost: true,
            optional_bytes: 0,
            retained_view_bytes: 0,
            cost_fits: true,
        })),
    },
    Case {
        name: CaseName("impossible_admission_does_not_mutate_even_dormant_payload_or_capacity"),
        word: Word::Succeeds,
        input: || Indexes::DormantShort,
        expect: Expect::Value(Observed::Refused {
            granted: false,
            unchanged: true,
            reclaimed_bytes: 0,
        }),
    },
    Case {
        name: CaseName("fifo_reclaims_only_enough_dormant_entries_and_preserves_survivor_order"),
        word: Word::Succeeds,
        input: || Indexes::TwoDormant,
        expect: Expect::Value(Observed::Fifo(Fifo {
            indexes: 2,
            granted: true,
            indexes_after: 1,
            survivor: [ColumnIndex(1)],
            capacity_kept: true,
            freed_the_first: true,
        })),
    },
    Case {
        name: CaseName("owner_reclaims_the_oldest_complete_index_before_newer_indexes"),
        word: Word::Succeeds,
        input: || Indexes::TwoDormantViews,
        expect: Expect::Value(Observed::Oldest(Oldest {
            first_view_bytes: true,
            indexes: 2,
            second_adds_view_bytes: true,
            granted: true,
            oldest_alive: false,
            newer_alive: true,
            indexes_after: 1,
            reclaimed_indexes: 1,
            freed_only_the_first: true,
        })),
    },
    Case {
        name: CaseName("enabling_graph_demand_does_not_reserve_unused_ordinary_capacity"),
        word: Word::Succeeds,
        input: || Indexes::UnderTheLimit,
        expect: Expect::Value(Observed::Fits {
            limit: LIMIT,
            granted: true,
            bytes_unchanged: true,
            indexes: 1,
        }),
    },
];

/// Builds the owner's indexes, sets its limit and asks for a graph's room.
fn subject(input: Input) -> Result<Value, Failure> {
    let columns = columns();
    let rows = rows(&columns);
    let mut fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    Ok(match input {
        Indexes::Held => {
            let handle = fixture
                .index(Relation::Function, &rows, &[ColumnIndex(0)])
                .expect(constants::OWNER_ACQUIRED_INDEX);
            let before = fixture.snapshot();
            let cost = before.index_payloads[0];
            fixture.set_limit(before.bytes + cost / 2);
            let granted_while_held = fixture.reclaim(cost);
            let pinned = fixture.snapshot();
            drop(handle);
            let granted_after_release = fixture.reclaim(cost);
            let reclaimed = fixture.snapshot();
            Observed::Held(Held {
                indexes: before.indexes,
                granted_while_held,
                bytes_kept_while_held: pinned.bytes == before.bytes,
                indexes_while_held: pinned.indexes,
                pinned_refusals: pinned.counts.pinned_refusals,
                granted_after_release,
                credited_once: reclaimed.bytes
                    + usize::try_from(reclaimed.counts.reclaimed_bytes)
                        .expect("a byte count fits in usize")
                    == before.bytes,
                indexes_after: reclaimed.indexes,
                capacity_kept: reclaimed.owner_entries_capacity == before.owner_entries_capacity,
                reclaimed_indexes: reclaimed.counts.reclaimed_indexes,
                reclaimed_the_cost: reclaimed.counts.reclaimed_bytes
                    == u64::try_from(cost).expect("a byte count fits in u64"),
                optional_bytes: reclaimed.optional_bytes,
                retained_view_bytes: reclaimed.retained_view_bytes,
                cost_fits: reclaimed.bytes + cost <= reclaimed.limit,
            })
        }
        Indexes::DormantShort => {
            drop(
                fixture
                    .index(Relation::Function, &rows, &[ColumnIndex(0)])
                    .expect(constants::OWNER_ACQUIRED_INDEX),
            );
            let before = fixture.snapshot();
            fixture.set_limit(before.bytes);
            let granted = fixture.reclaim(before.index_payloads[0] + 1);
            let after = fixture.snapshot();
            Observed::Refused {
                granted,
                unchanged: (after.bytes, after.indexes, after.owner_entries_capacity)
                    == (before.bytes, before.indexes, before.owner_entries_capacity),
                reclaimed_bytes: after.counts.reclaimed_bytes,
            }
        }
        Indexes::TwoDormant => {
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
            let first = before.index_payloads[0];
            fixture.set_limit(before.bytes);
            let granted = fixture.reclaim(first);
            let after = fixture.snapshot();
            Observed::Fifo(Fifo {
                indexes: before.indexes,
                granted,
                indexes_after: after.indexes,
                survivor: after.index_columns[0]
                    .clone()
                    .try_into()
                    .map_err(|columns| format!("survivor columns {columns:?}"))?,
                capacity_kept: after.owner_entries_capacity == before.owner_entries_capacity,
                freed_the_first: after.bytes <= before.bytes - first,
            })
        }
        Indexes::TwoDormantViews => {
            let first = fixture
                .index(Relation::Function, &rows, &[ColumnIndex(0)])
                .expect(constants::FIRST_OWNER_VIEW);
            let first_storage = Arc::downgrade(first.storage());
            let first_view_bytes = fixture.snapshot().retained_view_bytes;
            drop(first);
            let second = fixture
                .index(Relation::Function, &rows, &[ColumnIndex(1)])
                .expect(constants::SECOND_OWNER_VIEW);
            let second_storage = Arc::downgrade(second.storage());
            drop(second);
            let before = fixture.snapshot();
            fixture.set_limit(before.bytes);
            let granted = fixture.reclaim(first_view_bytes);
            let after = fixture.snapshot();
            Observed::Oldest(Oldest {
                first_view_bytes: first_view_bytes > 0,
                indexes: before.indexes,
                second_adds_view_bytes: before.retained_view_bytes > first_view_bytes,
                granted,
                oldest_alive: first_storage.upgrade().is_some(),
                newer_alive: second_storage.upgrade().is_some(),
                indexes_after: after.indexes,
                reclaimed_indexes: after.counts.reclaimed_indexes,
                freed_only_the_first: after.retained_view_bytes
                    == before.retained_view_bytes - first_view_bytes,
            })
        }
        Indexes::UnderTheLimit => {
            drop(
                fixture
                    .index(Relation::Function, &rows, &[ColumnIndex(0)])
                    .expect(constants::OWNER_ACQUIRED_INDEX),
            );
            let before = fixture.snapshot();
            let granted = fixture.reclaim(1);
            let after = fixture.snapshot();
            Observed::Fits {
                limit: before.limit,
                granted,
                bytes_unchanged: after.bytes == before.bytes,
                indexes: after.indexes,
            }
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
