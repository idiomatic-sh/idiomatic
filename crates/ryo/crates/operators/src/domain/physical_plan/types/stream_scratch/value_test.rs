//! A settled stage is empty and keeps no more room in its chunk and its kept right rows than the bound; room within the
//! bound is kept for the next run, as are its own index and repeats; a file's end gives those to the worker's index pool,
//! and past a file no chain keeps more room in any table than the cap.
use super::value::{ChainScratch, STREAM_CHUNK_CELLS, StageScratch, StreamScratch};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::kept_room::constants::KEPT_ROOM;
use crate::domain::kept_room::types::{KeptRoom as _, PoolSize, Room};
use crate::domain::physical_plan::types::keyed_rows::value::{KeyedRows, NO_ROW};
use crate::domain::physical_plan::types::row_set::value::RowSet;
use crate::domain::physical_plan::types::shared_index::value::{IndexPools, IndexRows};
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
    stage.settled();
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
    /// Its stage's own index of 1 Ki rows and own repeats of room for 512; then the file ends.
    OwnIndexes,
}

/// What a chain keeps after it settles and after the file ends, and what the worker's index pool gained.
#[derive(Debug, PartialEq, Eq)]
enum Kept {
    /// Whether the most room any of its buffers keeps past the file is within `KEPT_ROOM`, the most room its stages' own
    /// index and repeats keep once it settled and once the file ended, and the tables the pools gained.
    Chain {
        within: bool,
        settled: Room,
        ended: Room,
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
            settled: Room(0),
            ended: Room(0),
            pooled: PoolSize(0),
        }),
    },
    Case {
        name: CaseName("a_files_end_gives_each_stages_index_and_repeats_to_the_pool"),
        word: Word::Succeeds,
        input: || Chain::OwnIndexes,
        expect: Expect::Value(Kept::Chain {
            within: true,
            settled: Room(1 << 10),
            ended: Room(0),
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
    let mut pools = IndexPools::default();
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
            stage.own.index = IndexRows::Keyed(keyed(1 << 10));
            stage.own.repeats = RowSet::with_room(Room(1 << 9));
        }
    }
    scratch.stages.push(stage);
    let own = |stream: &StreamScratch| {
        stream
            .levels
            .iter()
            .flat_map(|level| &level.stages)
            .map(|stage| stage.own.index.room().max(stage.own.repeats.room()))
            .max()
            .unwrap_or_default()
    };
    scratch.settled();
    let mut stream = StreamScratch {
        levels: vec![scratch],
    };
    let settled = own(&stream);
    let before = pools.keyed.len().0 + pools.rows.len().0;
    stream.given_back(&mut pools);
    stream.kept();
    Ok(Kept::Chain {
        within: stream.room() <= Room(KEPT_ROOM),
        settled,
        ended: own(&stream),
        pooled: PoolSize(pools.keyed.len().0 + pools.rows.len().0 - before),
    })
}

#[test]
fn settles_chains() {
    run(CHAIN_CASES, Word::Succeeds, chain);
}

/// The keyed-row tables and the row sets the worker's pools gained when the file ended, its stage's own index a row set.
#[derive(Debug, PartialEq, Eq)]
struct GivenBack {
    keyed: PoolSize,
    sets: PoolSize,
}

const ROWS_INDEX_CASES: &[Case<(), GivenBack, Failure>] = &[Case {
    name: CaseName("a_files_end_gives_a_stages_rows_index_to_the_row_set_pool"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(GivenBack {
        keyed: PoolSize(0),
        sets: PoolSize(1),
    }),
}];

fn rows_index((): ()) -> Result<GivenBack, Failure> {
    let mut pools = IndexPools::default();
    let mut stage = StageScratch::default();
    stage.own.index = IndexRows::Rows(RowSet::with_room(Room(1 << 9)));
    let mut stream = StreamScratch {
        levels: vec![ChainScratch {
            stages: vec![stage],
            ..ChainScratch::default()
        }],
    };
    stream.given_back(&mut pools);
    Ok(GivenBack {
        keyed: pools.keyed.len(),
        sets: pools.rows.len(),
    })
}

#[test]
fn gives_back_rows_indexes() {
    run(ROWS_INDEX_CASES, Word::Succeeds, rows_index);
}
