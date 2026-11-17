//! Buffers answer no rows for a node they do not hold, and keep answering none once it is released; `retain` keeps the rows its
//! predicate keeps, in their order, and every row when it keeps them all. A released node's table goes to the worker's pool,
//! unless it is past the pool's cap on its room. A released or ended shared index gives its rows to the worker's index pool,
//! which keeps no more than its cap of them; the file's end gives the worker's own index, second index and repeats to the
//! pools of their kinds.
use super::value::FileBuffers;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::kept_room::constants::{KEPT_ROOM, POOL_INDEXES};
use crate::domain::kept_room::types::{KeptRoom as _, PoolSize, Room};
use crate::domain::physical_plan::types::IndexPlace;
use crate::domain::physical_plan::types::keyed_rows::value::{KeyedRows, NO_ROW};
use crate::domain::physical_plan::types::row_set::value::RowSet;
use crate::domain::physical_plan::types::shared_index::value::{IndexRows, SharedIndex};
use crate::domain::physical_plan::types::{NodeRows, PlanNodeId};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

#[derive(Debug, PartialEq)]
enum Held {
    Absent,
    /// The numbers of the rows held, in order.
    Materialized(Cow<'static, [i64]>),
}

type Input = Released;
type Value = Held;
type Failure = ();

/// What happens to node 0 before it is asked for.
#[derive(Clone, Copy)]
enum Released {
    /// It is never run.
    Never,
    /// It is released.
    Before,
    /// It holds 1, 2, 3, 2 and keeps every row.
    KeepsAll,
    /// It holds 1, 2, 3, 2 and keeps the rows that are not 2.
    DropsTwos,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_never_run_has_no_rows"),
        word: Word::Succeeds,
        input: || Released::Never,
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("a_released_node_has_no_rows"),
        word: Word::Succeeds,
        input: || Released::Before,
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("retaining_every_row_keeps_every_row_in_order"),
        word: Word::Succeeds,
        input: || Released::KeepsAll,
        expect: Expect::Value(Held::Materialized(Cow::Borrowed(&[1, 2, 3, 2]))),
    },
    Case {
        name: CaseName("retaining_drops_the_rows_it_does_not_keep_and_keeps_the_others_in_order"),
        word: Word::Succeeds,
        input: || Released::DropsTwos,
        expect: Expect::Value(Held::Materialized(Cow::Borrowed(&[1, 3]))),
    },
];

fn subject(released: Input) -> Result<Value, Failure> {
    let mut buffers = FileBuffers::default();
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for number in [1, 2, 3, 2] {
        push_row(&mut table, &[Cell::Number(Integer(number))]);
    }
    match released {
        Released::Never => {}
        Released::Before => buffers.release(PlanNodeId(0)),
        Released::KeepsAll => {
            buffers.nodes = vec![Some(table)];
            buffers.retain(PlanNodeId(0), |_| PredicateOutcome::Holds);
        }
        Released::DropsTwos => {
            buffers.nodes = vec![Some(table)];
            buffers.retain(PlanNodeId(0), |row| {
                if row == [Cell::Number(Integer(2))] {
                    PredicateOutcome::Fails
                } else {
                    PredicateOutcome::Holds
                }
            });
        }
    }
    Ok(match buffers.rows(PlanNodeId(0)) {
        NodeRows::Materialized(table) => Held::Materialized(Cow::Owned(
            table
                .cells
                .iter()
                .filter_map(|cell| {
                    if let Cell::Number(Integer(number)) = cell {
                        Some(*number)
                    } else {
                        None
                    }
                })
                .collect(),
        )),
        NodeRows::Absent => Held::Absent,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// The table of node 0 a pool case releases.
#[derive(Clone, Copy, Debug)]
enum Pooled {
    /// A table of a few rows.
    Small,
    /// A table of 128 Ki cells of room.
    TooManyCells,
}

/// The pool cases: how many tables the worker's pool keeps after node 0 is released.
const POOL_CASES: &[Case<Pooled, PoolSize, Failure>] = &[
    Case {
        name: CaseName("a_released_nodes_table_goes_to_the_pool"),
        word: Word::Succeeds,
        input: || Pooled::Small,
        expect: Expect::Value(PoolSize(1)),
    },
    Case {
        name: CaseName("a_table_too_large_is_dropped_not_pooled"),
        word: Word::Succeeds,
        input: || Pooled::TooManyCells,
        expect: Expect::Value(PoolSize(0)),
    },
];

fn pooled(given: Pooled) -> Result<PoolSize, Failure> {
    let mut buffers = FileBuffers::default();
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    push_row(&mut table, &[Cell::Number(Integer(1))]);
    if let Pooled::TooManyCells = given {
        table.cells.reserve_exact(1 << 17);
    }
    buffers.nodes = vec![Some(table)];
    buffers.release(PlanNodeId(0));
    Ok(buffers.pool.len())
}

#[test]
fn pools() {
    run(POOL_CASES, Word::Succeeds, pooled);
}

/// The shared indexes a worker holds before a file's end or a release gives them back.
#[derive(Clone, Copy, Debug)]
enum Shared {
    /// 1,000 indexes the open file opened, each of `KEPT_ROOM` rows; then the file ends.
    ManyEnded,
    /// One index of a few rows, released with its last probe.
    SmallReleased,
    /// One index of 100 Ki rows, released with its last probe.
    BigReleased,
}

/// The tables the worker's index pool keeps after the case, and the most room a shared index holds.
type Given = (PoolSize, Room);

const SHARED_CASES: &[Case<Shared, Given, Failure>] = &[
    Case {
        name: CaseName("a_worker_keeps_at_most_pool_indexes_past_a_file"),
        word: Word::Succeeds,
        input: || Shared::ManyEnded,
        expect: Expect::Value((PoolSize(POOL_INDEXES), Room(0))),
    },
    Case {
        name: CaseName("a_released_shared_index_holds_no_room"),
        word: Word::Succeeds,
        input: || Shared::SmallReleased,
        expect: Expect::Value((PoolSize(1), Room(0))),
    },
    Case {
        name: CaseName("a_released_shared_index_past_kept_room_is_dropped_not_pooled"),
        word: Word::Succeeds,
        input: || Shared::BigReleased,
        expect: Expect::Value((PoolSize(0), Room(0))),
    },
];

/// A shared index of `rows` rows.
fn index_of(rows: usize) -> SharedIndex {
    let mut keyed = KeyedRows::default();
    keyed.next.resize(rows, NO_ROW);
    SharedIndex {
        rows: IndexRows::Keyed(keyed),
        ..SharedIndex::default()
    }
}

fn given_back(given: Shared) -> Result<Given, Failure> {
    let mut buffers = FileBuffers::default();
    match given {
        Shared::ManyEnded => {
            buffers.shared.indexes = (0..1_000).map(|_| index_of(KEPT_ROOM)).collect();
            buffers.shared.opened = (0..1_000).map(IndexPlace).collect();
            buffers.ended();
        }
        Shared::SmallReleased | Shared::BigReleased => {
            let rows = if let Shared::SmallReleased = given {
                8
            } else {
                100 << 10
            };
            buffers.shared.indexes = vec![index_of(rows)];
            buffers.shared.closing = vec![Some(IndexPlace(0))];
            buffers.release(PlanNodeId(0));
        }
    }
    Ok((
        buffers.pools.keyed.len(),
        buffers
            .shared
            .indexes
            .iter()
            .map(|index| index.rows.room())
            .max()
            .unwrap_or_default(),
    ))
}

#[test]
fn gives_back_shared_indexes() {
    run(SHARED_CASES, Word::Succeeds, given_back);
}

/// The room a worker's own index, second index and repeats keep once the file ended, and the tables and sets its pools gained:
/// each is given to the pool of its kind, none keeps room in place.
#[derive(Debug, PartialEq, Eq)]
struct OwnGivenBack {
    index: Room,
    second: Room,
    repeats: Room,
    keyed: PoolSize,
    sets: PoolSize,
}

const OWN_CASES: &[Case<(), OwnGivenBack, Failure>] = &[Case {
    name: CaseName("a_files_end_gives_the_workers_own_index_second_and_repeats_to_their_pools"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(OwnGivenBack {
        index: Room(0),
        second: Room(0),
        repeats: Room(0),
        keyed: PoolSize(2),
        sets: PoolSize(1),
    }),
}];

fn own_given_back((): ()) -> Result<OwnGivenBack, Failure> {
    let mut buffers = FileBuffers::default();
    buffers.index = KeyedRows::with_room(Room(64));
    buffers.second = IndexRows::Keyed(KeyedRows::with_room(Room(64)));
    buffers.repeats = RowSet::with_room(Room(64));
    buffers.ended();
    Ok(OwnGivenBack {
        index: buffers.index.room(),
        second: buffers.second.room(),
        repeats: buffers.repeats.room(),
        keyed: buffers.pools.keyed.len(),
        sets: buffers.pools.rows.len(),
    })
}

#[test]
fn gives_back_own_indexes() {
    run(OWN_CASES, Word::Succeeds, own_given_back);
}
