//! A settled stage is empty and keeps no more room in its chunk and its kept right rows than the bound; room within the
//! bound is kept for the next run.
use super::value::{STREAM_CHUNK_CELLS, StageScratch};
use crate::cases::{Case, CaseName, Expect, Word, run};
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
