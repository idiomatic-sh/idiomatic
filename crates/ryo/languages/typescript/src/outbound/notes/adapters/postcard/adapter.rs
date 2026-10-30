//! The notes port, implemented with `postcard`: a note is written as postcard writes its wire shape, and bytes that are not exactly one
//! such shape, cut short or followed by more, are nothing. The library's names stop in `schemas.rs`, with the shapes.

use super::schemas::{note_from, write_as};
use crate::domain::fact_notes::types::Note;
use crate::outbound::notes::errors::NotesError;
use crate::outbound::notes::interface::NotesPort;
use ryo::languages::Bytes;

/// The `postcard` notes adapter.
#[derive(Debug)]
pub(crate) struct NotesPostcardAdapter;

impl NotesPort for NotesPostcardAdapter {
    fn write(&self, note: &Note, out: &mut Bytes) {
        write_as(note, out);
    }

    fn note_in(&self, bytes: &Bytes) -> Result<Note, NotesError> {
        note_from(bytes)
    }
}
