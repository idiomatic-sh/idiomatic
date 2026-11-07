//! What postcard writes and reads, which the adapter does not control, and its conversion from and into the language's own types: the
//! wire shape of a file's note, written from the note without making a copy of it, and read into one.

use serde::ser::SerializeStruct as _;
use serde::{Deserialize, Serialize, Serializer};

use super::constants::{END, IMPORT, IMPORT_FIELDS, IMPORTS, NOTE, NOTE_FIELDS, PATH, START, TEXT};
use super::types::Leftover;
use crate::domain::fact_notes::types::{ImportNote, ImportPath, Note, Spelling};
use crate::outbound::notes::errors::NotesError;
use ryo::languages::{Bytes, Offset};

/// A text, as it is written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TextSchema(String);

/// A place in a file, in bytes, as it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct OffsetSchema(u32);

/// The note of a file, as it is read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct NoteSchema {
    /// The packages the file imports, in source order.
    imports: Vec<ImportSchema>,
}

/// One package a file imports, as it is read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct ImportSchema {
    /// The path it names, decoded.
    path: TextSchema,
    /// Where the spec starts.
    start: OffsetSchema,
    /// Where it ends.
    end: OffsetSchema,
    /// The spec as the file writes it, its whitespace left out.
    text: TextSchema,
}

impl From<NoteSchema> for Note {
    fn from(schema: NoteSchema) -> Self {
        Self {
            imports: schema
                .imports
                .into_iter()
                .map(|each| ImportNote {
                    path: ImportPath(each.path.0.into_boxed_str()),
                    start: Offset(each.start.0),
                    end: Offset(each.end.0),
                    text: Spelling(each.text.0.into_boxed_str()),
                })
                .collect(),
        }
    }
}

/// The note of a file, as it is written: a view of the note itself, so that writing it makes no copy of it.
pub(super) struct NoteViewSchema<'v>(&'v Note);

/// One import, as it is written.
struct ImportViewSchema<'v>(&'v ImportNote);

/// The imports of a note, as they are written.
struct ImportsViewSchema<'v>(&'v [ImportNote]);

impl Serialize for NoteViewSchema<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut note = serializer.serialize_struct(NOTE, NOTE_FIELDS)?;
        note.serialize_field(IMPORTS, &ImportsViewSchema(&self.0.imports))?;
        note.end()
    }
}

impl Serialize for ImportsViewSchema<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.0.iter().map(ImportViewSchema))
    }
}

impl Serialize for ImportViewSchema<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut import = serializer.serialize_struct(IMPORT, IMPORT_FIELDS)?;
        import.serialize_field(PATH, &*self.0.path.0)?;
        import.serialize_field(START, &self.0.start.0)?;
        import.serialize_field(END, &self.0.end.0)?;
        import.serialize_field(TEXT, &*self.0.text.0)?;
        import.end()
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
    out.0 = postcard::to_extend(&NoteViewSchema(note), buffer).unwrap_or_default();
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
