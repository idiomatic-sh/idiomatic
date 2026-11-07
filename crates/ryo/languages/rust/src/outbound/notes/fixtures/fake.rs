//! The fake of the `notes` port: what the language's tests stand in for it with.

use super::super::errors::NotesError;
use super::super::interface::NotesPort;
use crate::domain::fact_notes::types::Note;
use ryo::languages::Bytes;

/// What the language's tests stand in for `notes` with: it writes one byte for anything and reads nothing back.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NotesFake;

impl NotesPort for NotesFake {
    fn write(&self, _note: &Note<'_>, out: &mut Bytes) {
        out.0.clear();
        out.0.push(1);
    }

    fn note_in<'a>(&self, _bytes: &'a Bytes) -> Result<Note<'a>, NotesError> {
        Err(NotesError::NotANote)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
