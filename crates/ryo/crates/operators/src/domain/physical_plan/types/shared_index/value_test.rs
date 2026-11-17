//! A shared index given back holds no key, no row and no probe still to come, so the next probe builds it anew; an index the
//! open file opened, closed and opened again is listed once, and the file's end gives it back and lists none. Each kind of
//! index rows goes back to the pool of its kind, and is readied from it, its room of the other kind given back first.
use super::value::{IndexKind, IndexRows, SharedIndex, SharedIndexes};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::kept_room::types::{PoolSize, Room};
use crate::domain::physical_plan::types::keyed_rows::value::{
    IndexKeyHash, IndexedRow, KeyedRows, KeyedRun, NO_ROW,
};
use crate::domain::physical_plan::types::row_set::value::RowSet;
use crate::domain::physical_plan::types::shared_index::value::IndexPools;
use crate::domain::physical_plan::types::{IndexPlace, NodeReaders};
use ryo_values::domain::rows::types::RowCount;

/// What a case does to an index of one key and one row, two probes still to come.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Gives it back.
    GivenBack,
    /// Opens it among two indexes, gives it back, opens it again: the indexes listed.
    OpenedTwice,
    /// Opens it among two indexes, then ends the file.
    Ended,
}
type Input = Given;
/// The keys and rows the index holds, its probes still to come, and how many indexes are listed as opened.
type Value = (usize, usize, NodeReaders, usize);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_index_given_back_holds_nothing_for_the_next_file"),
        word: Word::Succeeds,
        input: || Given::GivenBack,
        expect: Expect::Value((0, 0, NodeReaders(0), 0)),
    },
    Case {
        name: CaseName("an_index_opened_twice_in_a_file_is_listed_once"),
        word: Word::Succeeds,
        input: || Given::OpenedTwice,
        expect: Expect::Value((0, 0, NodeReaders(2), 1)),
    },
    Case {
        name: CaseName("a_files_end_empties_the_index_it_opened_and_lists_none"),
        word: Word::Succeeds,
        input: || Given::Ended,
        expect: Expect::Value((0, 0, NodeReaders(0), 0)),
    },
];

/// An index of one key and one row.
fn built() -> SharedIndex {
    let mut rows = KeyedRows::default();
    let hash = IndexKeyHash(7);
    let _: hashbrown::hash_table::OccupiedEntry<'_, KeyedRun> = rows.keys.insert_unique(
        hash.0,
        KeyedRun {
            hash,
            first: IndexedRow(0),
            last: IndexedRow(0),
            rows: RowCount(1),
        },
        |run| run.hash.0,
    );
    rows.next.push(NO_ROW);
    SharedIndex {
        rows: IndexRows::Keyed(rows),
        ..SharedIndex::default()
    }
}

fn subject(given: Input) -> Result<Value, Failure> {
    let mut pool = IndexPools::default();
    let mut shared = SharedIndexes {
        indexes: vec![SharedIndex::default(), built()],
        ..SharedIndexes::default()
    };
    let at = IndexPlace(1);
    match given {
        Given::GivenBack => {
            if let Some(index) = shared.indexes.get_mut(at.0) {
                index.remaining = NodeReaders(2);
                index.given_back(&mut pool);
            }
        }
        Given::OpenedTwice => {
            shared.opened(at, NodeReaders(2), &mut pool);
            if let Some(index) = shared.indexes.get_mut(at.0) {
                index.given_back(&mut pool);
            }
            shared.opened(at, NodeReaders(2), &mut pool);
        }
        Given::Ended => {
            shared.opened(at, NodeReaders(2), &mut pool);
            shared.ended(&mut pool);
        }
    }
    let index = shared.indexes.get(at.0).ok_or(())?;
    let IndexRows::Keyed(rows) = &index.rows else {
        return Err(());
    };
    Ok((
        rows.keys.len(),
        rows.next.len(),
        index.remaining,
        shared.opened.len(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// What a case does to an index's rows of one kind.
#[derive(Clone, Copy, Debug)]
enum Rows {
    /// A shared row-set index given back.
    RowSetGivenBack,
    /// A shared keyed index given back.
    KeyedGivenBack,
    /// Keyed rows of room readied as a row set, the row-set pool holding one set.
    KeyedReadiedAsRows,
}

/// The tables the keyed pool and the sets the row-set pool keep after it, the kind of the rows, and their room.
#[derive(Debug, PartialEq, Eq)]
struct Pooled {
    keyed: PoolSize,
    sets: PoolSize,
    kind: IndexKind,
    room: Room,
}

const ROWS_CASES: &[Case<Rows, Pooled, Failure>] = &[
    Case {
        name: CaseName("a_rows_index_is_given_back_to_the_row_set_pool"),
        word: Word::Succeeds,
        input: || Rows::RowSetGivenBack,
        expect: Expect::Value(Pooled {
            keyed: PoolSize(0),
            sets: PoolSize(1),
            kind: IndexKind::Rows,
            room: Room(0),
        }),
    },
    Case {
        name: CaseName("a_keyed_index_is_given_back_to_the_keyed_pool"),
        word: Word::Succeeds,
        input: || Rows::KeyedGivenBack,
        expect: Expect::Value(Pooled {
            keyed: PoolSize(1),
            sets: PoolSize(0),
            kind: IndexKind::Keyed,
            room: Room(0),
        }),
    },
    Case {
        name: CaseName("keyed_rows_readied_as_a_row_set_give_their_room_to_the_keyed_pool"),
        word: Word::Succeeds,
        input: || Rows::KeyedReadiedAsRows,
        expect: Expect::Value(Pooled {
            keyed: PoolSize(1),
            sets: PoolSize(0),
            kind: IndexKind::Rows,
            room: Room(112),
        }),
    },
];

fn rows_subject(given: Rows) -> Result<Pooled, Failure> {
    let mut pools = IndexPools::default();
    let mut index = SharedIndex {
        rows: match given {
            Rows::RowSetGivenBack => IndexRows::Rows(RowSet::with_room(Room(64))),
            Rows::KeyedGivenBack | Rows::KeyedReadiedAsRows => {
                IndexRows::Keyed(KeyedRows::with_room(Room(64)))
            }
        },
        ..SharedIndex::default()
    };
    match given {
        Rows::RowSetGivenBack | Rows::KeyedGivenBack => index.given_back(&mut pools),
        Rows::KeyedReadiedAsRows => {
            pools.rows.pooled(RowSet::with_room(Room(64)), PoolSize(1));
            index.rows.readied(IndexKind::Rows, &mut pools);
        }
    }
    Ok(Pooled {
        keyed: pools.keyed.len(),
        sets: pools.rows.len(),
        kind: index.rows.kind(),
        room: index.rows.room(),
    })
}

#[test]
fn pools_rows_by_kind() {
    run(ROWS_CASES, Word::Succeeds, rows_subject);
}
