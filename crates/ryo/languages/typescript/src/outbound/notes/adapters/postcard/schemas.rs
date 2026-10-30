//! What postcard writes and reads, which the adapter does not control, and its conversion from and into the language's own types: the
//! wire shape of a file's note, written from a view of the note that borrows its specifiers, and read into one that owns them.

use serde::{Deserialize, Serialize};

use super::types::Leftover;
use crate::domain::fact_notes::types::{ImportNote, ImportSpelling, Note, Specifier};
use crate::outbound::notes::errors::NotesError;
use ryo::languages::{Bytes, Offset};

/// A place in a file, in bytes, as it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct OffsetSchema(u32);

/// The note of a file, as it is read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct NoteSchema {
    /// The modules the file imports, in source order.
    imports: Vec<ImportSchema>,
}

/// One module a file imports, as it is read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct ImportSchema {
    /// The module it names, decoded.
    specifier: String,
    /// Where what imports it starts.
    start: OffsetSchema,
    /// Where it ends.
    end: OffsetSchema,
    /// The import as the file writes it, its whitespace left out.
    text: String,
}

/// The note of a file, as it is written: a view of the note, borrowing its specifiers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct NoteViewSchema<'a> {
    /// The modules the file imports, in source order.
    imports: Vec<ImportViewSchema<'a>>,
}

/// One module a file imports, as it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(super) struct ImportViewSchema<'a> {
    /// The module it names, decoded.
    specifier: &'a str,
    /// Where what imports it starts.
    start: OffsetSchema,
    /// Where it ends.
    end: OffsetSchema,
    /// The import as the file writes it, its whitespace left out.
    text: &'a str,
}

impl From<NoteSchema> for Note {
    fn from(schema: NoteSchema) -> Self {
        Self {
            imports: schema
                .imports
                .into_iter()
                .map(|each| ImportNote {
                    specifier: Specifier(each.specifier.into_boxed_str()),
                    start: Offset(each.start.0),
                    end: Offset(each.end.0),
                    text: ImportSpelling(each.text.into_boxed_str()),
                })
                .collect(),
        }
    }
}

impl<'a> From<&'a Note> for NoteViewSchema<'a> {
    fn from(note: &'a Note) -> Self {
        Self {
            imports: note
                .imports
                .iter()
                .map(|each| ImportViewSchema {
                    specifier: &each.specifier.0,
                    start: OffsetSchema(each.start.0),
                    end: OffsetSchema(each.end.0),
                    text: &each.text.0,
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
    out.0 = postcard::to_extend(&NoteViewSchema::from(note), buffer).unwrap_or_default();
}

/// The note `bytes` hold, when they hold exactly one.
pub(super) fn note_from(bytes: &Bytes) -> Result<Note, NotesError> {
    let (schema, rest) = postcard::take_from_bytes::<NoteSchema>(&bytes.0)
        .map_err(|_not_a_note| NotesError::NotANote)?;
    match left(rest) {
        Leftover::Nothing => Ok(Note::from(schema)),
        Leftover::More => Err(NotesError::NotANote),
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
