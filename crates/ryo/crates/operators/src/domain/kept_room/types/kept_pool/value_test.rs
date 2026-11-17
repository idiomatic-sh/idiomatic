//! A pool keeps a given-back buffer's room for its next use, emptied; it drops one past its caps or one that holds no room,
//! and keeps no more buffers than its most.
use super::value::KeptPool;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::kept_room::constants::{KEPT_ROOM, POOL_TABLE_KINDS, POOL_TABLES};
use crate::domain::kept_room::types::PoolSize;
use crate::domain::physical_plan::types::keyed_rows::value::{KeyedRows, NO_ROW};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::rows::types::{RowCount, Table};

/// The node tables a case gives the pool.
#[derive(Clone, Copy, Debug)]
enum Tables {
    /// One table of a row, within both caps.
    Small,
    /// One table of room for more cells than the cap.
    ManyCells,
    /// One table of room for more kinds than the cap.
    ManyKinds,
    /// More small tables than the pool keeps.
    PastTheCount,
}
type Failure = ();

/// What the pool keeps after the case gave it its buffers, and the buffer it hands out next.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// How many buffers it keeps, whether each is empty and within the caps, and whether the next it hands out is empty.
    Kept {
        size: PoolSize,
        within: PredicateOutcome,
        fresh_empty: bool,
    },
}

const TABLE_CASES: &[Case<Tables, Observation, Failure>] = &[
    Case {
        name: CaseName("a_dropped_table_is_kept_emptied_for_the_next_node"),
        word: Word::Succeeds,
        input: || Tables::Small,
        expect: Expect::Value(Observation::Kept {
            size: PoolSize(1),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
    Case {
        name: CaseName("a_table_of_too_many_cells_is_dropped_not_pooled"),
        word: Word::Succeeds,
        input: || Tables::ManyCells,
        expect: Expect::Value(Observation::Kept {
            size: PoolSize(0),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
    Case {
        name: CaseName("a_table_of_too_many_kinds_is_dropped_not_pooled"),
        word: Word::Succeeds,
        input: || Tables::ManyKinds,
        expect: Expect::Value(Observation::Kept {
            size: PoolSize(0),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
    Case {
        name: CaseName("the_pool_keeps_no_more_tables_than_its_cap"),
        word: Word::Succeeds,
        input: || Tables::PastTheCount,
        expect: Expect::Value(Observation::Kept {
            size: PoolSize(POOL_TABLES),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
];

/// The outcome of a check of a kept buffer.
fn outcome(holds: bool) -> PredicateOutcome {
    if holds {
        PredicateOutcome::Holds
    } else {
        PredicateOutcome::Fails
    }
}

/// A table of one number row.
fn small() -> Table {
    Table {
        kinds: vec![CellKind::Number],
        cells: vec![Cell::Number(Integer(1))],
        count: RowCount(1),
    }
}

fn tables(given: Tables) -> Result<Observation, Failure> {
    let mut pool: KeptPool<Table> = KeptPool::default();
    let most = PoolSize(POOL_TABLES);
    match given {
        Tables::Small => pool.pooled(small(), most),
        Tables::ManyCells => {
            let mut table = small();
            table.cells.reserve_exact(KEPT_ROOM * 32);
            pool.pooled(table, most);
        }
        Tables::ManyKinds => {
            let mut table = small();
            table.kinds.reserve_exact(POOL_TABLE_KINDS * 2);
            pool.pooled(table, most);
        }
        Tables::PastTheCount => {
            for _ in 0..POOL_TABLES + 44 {
                pool.pooled(small(), most);
            }
        }
    }
    let size = pool.len();
    let within = pool.within_caps(|table| {
        outcome(table.cells.is_empty() && table.kinds.is_empty() && table.count == RowCount(0))
    });
    let fresh = pool.fresh();
    Ok(Observation::Kept {
        size,
        within,
        fresh_empty: fresh.cells.is_empty() && fresh.kinds.is_empty() && fresh.count == RowCount(0),
    })
}

#[test]
fn keeps_tables() {
    run(TABLE_CASES, Word::Succeeds, tables);
}

/// The keyed-row tables a case gives a pool that keeps at most two.
#[derive(Clone, Copy, Debug)]
enum Indexes {
    /// Three tables of a few keys' room.
    Three,
    /// One table of room past `KEPT_ROOM`.
    PastItsRoom,
    /// One table that holds no room.
    NoRoom,
}

const INDEX_CASES: &[Case<Indexes, Observation, Failure>] = &[
    Case {
        name: CaseName("a_pool_keeps_no_more_than_its_most"),
        word: Word::Succeeds,
        input: || Indexes::Three,
        expect: Expect::Value(Observation::Kept {
            size: PoolSize(2),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
    Case {
        name: CaseName("a_buffer_past_its_room_is_not_pooled"),
        word: Word::Succeeds,
        input: || Indexes::PastItsRoom,
        expect: Expect::Value(Observation::Kept {
            size: PoolSize(0),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
    Case {
        name: CaseName("a_buffer_of_no_room_is_not_pooled"),
        word: Word::Succeeds,
        input: || Indexes::NoRoom,
        expect: Expect::Value(Observation::Kept {
            size: PoolSize(0),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
];

/// A keyed-row table of `room` rows' room, one row held.
fn rows_of(room: usize) -> KeyedRows {
    let mut rows = KeyedRows::default();
    rows.next.reserve_exact(room);
    rows.next.push(NO_ROW);
    rows
}

fn indexes(given: Indexes) -> Result<Observation, Failure> {
    let mut pool: KeptPool<KeyedRows> = KeptPool::default();
    let most = PoolSize(2);
    match given {
        Indexes::Three => {
            for _ in 0..3 {
                pool.pooled(rows_of(8), most);
            }
        }
        Indexes::PastItsRoom => pool.pooled(rows_of(KEPT_ROOM * 2), most),
        Indexes::NoRoom => pool.pooled(KeyedRows::default(), most),
    }
    let size = pool.len();
    let within = pool.within_caps(|rows| outcome(rows.keys.is_empty() && rows.next.is_empty()));
    let fresh = pool.fresh();
    Ok(Observation::Kept {
        size,
        within,
        fresh_empty: fresh.keys.is_empty() && fresh.next.is_empty(),
    })
}

#[test]
fn keeps_indexes() {
    run(INDEX_CASES, Word::Succeeds, indexes);
}
