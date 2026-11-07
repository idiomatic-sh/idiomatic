//! What postcard writes and reads, which the adapter does not control, and its conversion from and into the language's own types: the
//! wire shape of a file's note, written from the note's own text and read with its specifiers borrowed from the bytes.

use serde::{Deserialize, Serialize};

use super::types::Leftover;
use crate::domain::fact_notes::types::{ImportNote, Note, Specifier, Spelling};
use crate::outbound::notes::errors::NotesError;
use ryo::languages::{Bytes, Offset};

/// A place in a file, in bytes, as it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct OffsetSchema(u32);

/// The note of a file, as it is written and read: the specifiers borrow the note, or the bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct NoteSchema<'a> {
    /// The modules the file imports, in source order.
    #[serde(borrow)]
    imports: Vec<ImportSchema<'a>>,
}

/// One module a file imports, as it is written and read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ImportSchema<'a> {
    /// The module it names, decoded.
    specifier: &'a str,
    /// Where what imports it starts, in bytes.
    start: OffsetSchema,
    /// Where it ends.
    end: OffsetSchema,
    /// The import as the file writes it, its whitespace left out.
    text: &'a str,
}

impl<'a> From<&'a Note> for NoteSchema<'a> {
    fn from(note: &'a Note) -> Self {
        Self {
            imports: note
                .imports
                .iter()
                .map(|import| ImportSchema {
                    specifier: &import.specifier.0,
                    start: OffsetSchema(import.start.0),
                    end: OffsetSchema(import.end.0),
                    text: &import.text.0,
                })
                .collect(),
        }
    }
}

impl From<NoteSchema<'_>> for Note {
    fn from(schema: NoteSchema<'_>) -> Self {
        Self {
            imports: schema
                .imports
                .into_iter()
                .map(|import| ImportNote {
                    specifier: Specifier(import.specifier.into()),
                    start: Offset(import.start.0),
                    end: Offset(import.end.0),
                    text: Spelling(import.text.into()),
                })
                .collect(),
        }
    }
}

/// Whether `rest`, what postcard left, holds anything.
fn left(rest: &[u8]) -> Leftover {
    if rest.is_empty() {
        Leftover::Nothing
    } else {
        Leftover::More
    }
}

/// Writes `note` into `out`, in place of what it held.
pub(super) fn write_as(note: &Note, out: &mut Bytes) {
    let mut buffer = std::mem::take(&mut out.0);
    buffer.clear();
    out.0 = postcard::to_extend(&NoteSchema::from(note), buffer).unwrap_or_default();
}

/// The note `bytes` hold, when they hold exactly one.
pub(super) fn note_from(bytes: &Bytes) -> Result<Note, NotesError> {
    let (schema, rest) = postcard::take_from_bytes::<NoteSchema<'_>>(&bytes.0)
        .map_err(|_not_a_note| NotesError::NotANote)?;
    match left(rest) {
        Leftover::Nothing => Ok(Note::from(schema)),
        Leftover::More => Err(NotesError::NotANote),
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
