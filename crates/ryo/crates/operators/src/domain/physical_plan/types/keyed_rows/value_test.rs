//! A cleared table holds no key and no row, and keeps its room for the next rows.
use super::value::{IndexKeyHash, IndexedRow, KeyedRows, KeyedRun};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::types::RowCount;

/// How many keyed rows the table held before it was cleared.
type Input = u32;
type Value = Observation;
type Failure = ();

/// What a cleared table holds.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// Whether it holds no key and no row, and whether it kept its room for as many rows.
    Cleared { empty: bool, room: bool },
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_cleared_table_holds_nothing_and_keeps_its_room"),
    word: Word::Succeeds,
    input: || 64,
    expect: Expect::Value(Observation::Cleared {
        empty: true,
        room: true,
    }),
}];

fn subject(rows: Input) -> Result<Value, Failure> {
    let mut table = KeyedRows::default();
    for row in 0..rows {
        table.next.push(IndexedRow(row));
        let run = KeyedRun {
            hash: IndexKeyHash(u64::from(row)),
            first: IndexedRow(row),
            last: IndexedRow(row),
            rows: RowCount(1),
        };
        let _slot = table.keys.insert_unique(run.hash.0, run, |key| key.hash.0);
    }
    table.cleared();
    Ok(Observation::Cleared {
        empty: table.keys.is_empty() && table.next.is_empty(),
        room: table.next.capacity() >= usize::try_from(rows).unwrap_or(usize::MAX)
            && table.keys.capacity() >= usize::try_from(rows).unwrap_or(usize::MAX),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
