//! A shared index given back holds no key, no row and no probe still to come, so the next probe builds it anew; an index the
//! open file opened, closed and opened again is listed once, and the file's end gives it back and lists none.
use super::value::{SharedIndex, SharedIndexes};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::kept_room::types::kept_pool::value::KeptPool;
use crate::domain::physical_plan::types::keyed_rows::value::{
    IndexKeyHash, IndexedRow, KeyedRun, NO_ROW,
};
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
    let mut index = SharedIndex::default();
    let hash = IndexKeyHash(7);
    let _: hashbrown::hash_table::OccupiedEntry<'_, KeyedRun> = index.rows.keys.insert_unique(
        hash.0,
        KeyedRun {
            hash,
            first: IndexedRow(0),
            last: IndexedRow(0),
            rows: RowCount(1),
        },
        |run| run.hash.0,
    );
    index.rows.next.push(NO_ROW);
    index
}

fn subject(given: Input) -> Result<Value, Failure> {
    let mut pool = KeptPool::default();
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
    Ok((
        index.rows.keys.len(),
        index.rows.next.len(),
        index.remaining,
        shared.opened.len(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
