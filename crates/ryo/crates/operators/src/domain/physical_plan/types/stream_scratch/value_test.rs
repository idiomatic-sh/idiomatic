//! A settled stage is empty and keeps no more room in its chunk and its kept right rows than the bound; room within the
//! bound is kept for the next run. Its own index and repeats go to the worker's index pool as its chain settles, and past a
//! file no chain keeps more room in any table than the cap.
use super::value::{ChainScratch, STREAM_CHUNK_CELLS, StageScratch, StreamScratch};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::kept_room::constants::KEPT_ROOM;
use crate::domain::kept_room::types::kept_pool::value::KeptPool;
use crate::domain::kept_room::types::{KeptRoom as _, PoolSize, Room};
use crate::domain::physical_plan::types::keyed_rows::value::{KeyedRows, NO_ROW};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::domain::rows::types::RowCount;

/// How many cells the stage's tables held before it settled.
type Input = usize;
type Value = Observation;
type Failure = ();

/// What a settled stage keeps.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// Whether its tables keep no more room than the bound, whether they keep the room they had within it, and whether they
    /// are empty.
    Settled {
        bounded: bool,
        kept: bool,
        empty: bool,
    },
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_settled_stage_keeps_no_more_room_than_the_bound"),
        word: Word::Succeeds,
        input: || STREAM_CHUNK_CELLS * 4,
        expect: Expect::Value(Observation::Settled {
            bounded: true,
            kept: true,
            empty: true,
        }),
    },
    Case {
        name: CaseName("a_settled_stage_keeps_room_within_the_bound"),
        word: Word::Succeeds,
        input: || STREAM_CHUNK_CELLS / 4,
        expect: Expect::Value(Observation::Settled {
            bounded: true,
            kept: true,
            empty: true,
        }),
    },
];

fn subject(cells: Input) -> Result<Value, Failure> {
    let mut stage = StageScratch::default();
    for table in [&mut stage.out, &mut stage.own.kept] {
        table
            .cells
            .extend((0..cells).map(|at| Cell::Number(Integer(i64::try_from(at).unwrap_or(0)))));
        table.count = RowCount(cells);
    }
    stage.settled(&mut KeptPool::default());
    let tables = [&stage.out, &stage.own.kept];
    Ok(Observation::Settled {
        bounded: tables
            .iter()
            .all(|table| table.cells.capacity() <= STREAM_CHUNK_CELLS),
        kept: tables
            .iter()
            .all(|table| table.cells.capacity() >= cells.min(STREAM_CHUNK_CELLS)),
        empty: tables
            .iter()
            .all(|table| table.cells.is_empty() && table.count == RowCount(0)),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// What a chain of one stage held before it settled.
#[derive(Clone, Copy, Debug)]
enum Chain {
    /// Its head, its stage's chunk and its stage's kept right rows, each of `STREAM_CHUNK_CELLS` cells; then the file ends.
    FullTables,
    /// Its stage's own index and own repeats, each of 1 Ki rows.
    OwnIndexes,
}

/// What a settled chain keeps, and what the worker's index pool gained.
#[derive(Debug, PartialEq, Eq)]
enum Kept {
    /// The most room any of its buffers keeps, whether that is within `KEPT_ROOM`, and the tables the pool gained.
    Chain {
        within: bool,
        own: Room,
        pooled: PoolSize,
    },
}

const CHAIN_CASES: &[Case<Chain, Kept, Failure>] = &[
    Case {
        name: CaseName("a_streamed_stage_keeps_at_most_kept_room_past_a_file"),
        word: Word::Succeeds,
        input: || Chain::FullTables,
        expect: Expect::Value(Kept::Chain {
            within: true,
            own: Room(0),
            pooled: PoolSize(0),
        }),
    },
    Case {
        name: CaseName("a_settled_stage_gives_its_index_and_repeats_to_the_pool"),
        word: Word::Succeeds,
        input: || Chain::OwnIndexes,
        expect: Expect::Value(Kept::Chain {
            within: true,
            own: Room(0),
            pooled: PoolSize(2),
        }),
    },
];

/// A keyed-row table of `rows` rows, each its own key's run's end.
fn keyed(rows: usize) -> KeyedRows {
    let mut keyed = KeyedRows::default();
    keyed.next.resize(rows, NO_ROW);
    keyed
}

fn chain(given: Chain) -> Result<Kept, Failure> {
    let mut pool = KeptPool::default();
    let mut scratch = ChainScratch::default();
    let mut stage = StageScratch::default();
    match given {
        Chain::FullTables => {
            for table in [&mut scratch.head, &mut stage.out, &mut stage.own.kept] {
                table.cells.extend(
                    (0..STREAM_CHUNK_CELLS)
                        .map(|at| Cell::Number(Integer(i64::try_from(at).unwrap_or(0)))),
                );
                table.count = RowCount(STREAM_CHUNK_CELLS);
            }
        }
        Chain::OwnIndexes => {
            stage.own.index = keyed(1 << 10);
            stage.own.repeats = keyed(1 << 10);
        }
    }
    scratch.stages.push(stage);
    let before = pool.len();
    scratch.settled(&mut pool);
    let own = scratch
        .stages
        .iter()
        .map(|stage| stage.own.index.room().max(stage.own.repeats.room()))
        .max()
        .unwrap_or_default();
    let mut stream = StreamScratch {
        levels: vec![scratch],
    };
    stream.kept();
    Ok(Kept::Chain {
        within: stream.room() <= Room(KEPT_ROOM),
        own,
        pooled: PoolSize(pool.len().0 - before.0),
    })
}

#[test]
fn settles_chains() {
    run(CHAIN_CASES, Word::Succeeds, chain);
}
