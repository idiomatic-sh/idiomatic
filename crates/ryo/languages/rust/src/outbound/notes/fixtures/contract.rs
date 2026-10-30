//! What every `notes` adapter must pass.

use super::super::errors::NotesError;
use super::super::interface::NotesPort;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::fact_notes::types::{Note, NoteFlag};
use ryo::languages::Bytes;

/// What a row of the contract asks of an adapter.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Probe {
    /// The note in no bytes at all.
    NoNote,
    /// The note in bytes that are not one.
    NotANote,
    /// Whether writing a note replaces what the room held.
    Replaces,
}

/// What a row of the contract is given: the probe it puts to the adapter.
type Input = Probe;
/// What the adapter answered, as text.
type Value = &'static str;
/// What a row of the contract expects when it is refused.
type Failure = NotesError;

/// What an answer is.
const SOMETHING: &str = "something";

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_bytes_hold_no_note"),
        word: Word::Refuses,
        input: || Probe::NoNote,
        expect: Expect::Refusal(|error| selects(matches!(error, NotesError::NotANote))),
    },
    Case {
        name: CaseName("bytes_that_are_not_a_note_hold_none"),
        word: Word::Refuses,
        input: || Probe::NotANote,
        expect: Expect::Refusal(|error| selects(matches!(error, NotesError::NotANote))),
    },
    Case {
        name: CaseName("writing_a_note_replaces_what_the_room_held"),
        word: Word::Succeeds,
        input: || Probe::Replaces,
        expect: Expect::Value(SOMETHING),
    },
];

/// The answer a row expects when a note is, or is not, read back.
fn shown(present: bool) -> Result<Value, NotesError> {
    if present {
        Ok(SOMETHING)
    } else {
        Err(NotesError::NotANote)
    }
}

/// Puts `probe` to `adapter` and says what it answered.
fn answer(adapter: &impl NotesPort, probe: Probe) -> Result<Value, NotesError> {
    match probe {
        Probe::NoNote => adapter.note_in(&Bytes::default()).map(|_| SOMETHING),
        Probe::NotANote => adapter.note_in(&Bytes(vec![2, 0, 0])).map(|_| SOMETHING),
        Probe::Replaces => {
            let mut room = Bytes(vec![9, 9, 9, 9, 9, 9, 9, 9]);
            adapter.write(
                &Note {
                    clean: NoteFlag(true),
                    declared: Vec::new(),
                    written: Vec::new(),
                },
                &mut room,
            );
            shown(!room.0.is_empty() && room.0 != vec![9, 9, 9, 9, 9, 9, 9, 9])
        }
    }
}

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl NotesPort) {
    run(CASES, Word::Succeeds, |probe| answer(adapter, probe));
    run(CASES, Word::Refuses, |probe| answer(adapter, probe));
}
