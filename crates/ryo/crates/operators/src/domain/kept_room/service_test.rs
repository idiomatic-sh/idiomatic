//! A buffer at a file's end is emptied; it keeps its room for the next file when that room is within the cap, and drops it
//! when a big file grew it past the cap.
use super::constants::KEPT_ROOM;
use super::service::{kept_list, kept_queue, kept_table};
use super::types::Room;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The buffer a case ends a file with.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// A list of room for the cap's items, one held.
    ListWithin,
    /// A list of room for one more item than the cap, one held.
    ListPast,
    /// A hash table of room for one more item than the cap, one held.
    TablePast,
    /// A queue of room for one more item than the cap, one held.
    QueuePast,
}
type Input = Given;
/// The room the buffer keeps after the file's end, and the items it holds.
type Value = (Room, usize);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_buffer_within_the_cap_keeps_its_room_emptied"),
        word: Word::Succeeds,
        input: || Given::ListWithin,
        expect: Expect::Value((Room(KEPT_ROOM), 0)),
    },
    Case {
        name: CaseName("a_list_past_the_cap_drops_its_room"),
        word: Word::Succeeds,
        input: || Given::ListPast,
        expect: Expect::Value((Room(0), 0)),
    },
    Case {
        name: CaseName("a_hash_table_past_the_cap_drops_its_room"),
        word: Word::Succeeds,
        input: || Given::TablePast,
        expect: Expect::Value((Room(0), 0)),
    },
    Case {
        name: CaseName("a_queue_past_the_cap_drops_its_room"),
        word: Word::Succeeds,
        input: || Given::QueuePast,
        expect: Expect::Value((Room(0), 0)),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    Ok(match given {
        Given::ListWithin | Given::ListPast => {
            let room = if matches!(given, Given::ListWithin) {
                KEPT_ROOM
            } else {
                KEPT_ROOM + 1
            };
            let mut list: Vec<u8> = Vec::with_capacity(room);
            list.push(1);
            kept_list(&mut list);
            (Room(list.capacity()), list.len())
        }
        Given::TablePast => {
            let mut table: hashbrown::HashTable<u64> =
                hashbrown::HashTable::with_capacity(KEPT_ROOM + 1);
            let _: hashbrown::hash_table::OccupiedEntry<'_, u64> =
                table.insert_unique(1, 1, |item| *item);
            kept_table(&mut table);
            (Room(table.capacity()), table.len())
        }
        Given::QueuePast => {
            let mut queue: std::collections::VecDeque<u8> =
                std::collections::VecDeque::with_capacity(KEPT_ROOM + 1);
            queue.push_back(1);
            kept_queue(&mut queue);
            (Room(queue.capacity()), queue.len())
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
