//! What postcard writes and reads, which the adapter does not control, and its conversion from and into the language's own types: the
//! wire shape of a file's note, written from a view of the note that copies none of its paths, and read into a note of its own.

use serde::{Deserialize, Serialize};

use super::types::Leftover;
use crate::domain::fact_notes::types::{DottedPath, Form, ImportNote, Level, Note, Spelling};
use crate::outbound::notes::errors::NotesError;
use ryo::languages::{Bytes, Offset};

/// A number, as it is written: how many packages up an import starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct LevelSchema(u32);

/// A place in a file, in bytes, as it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct OffsetSchema(u32);

/// A text, as it is read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct TextSchema(String);

/// What the dotted path of an import names, as it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum FormSchema {
    /// A module.
    Module,
    /// A module, or a name its last part's module defines.
    Name,
    /// Every name of a module.
    Everything,
}

/// The note of a file, as it is read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct NoteSchema {
    /// The imports the file writes, in source order.
    imports: Vec<ImportSchema>,
}

/// One import, as it is read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct ImportSchema {
    /// What the path names.
    form: FormSchema,
    /// How many packages up it starts.
    level: LevelSchema,
    /// The path, without its leading dots.
    path: TextSchema,
    /// Where the alias starts.
    start: OffsetSchema,
    /// Where it ends.
    end: OffsetSchema,
    /// The alias as the file writes it, its whitespace left out.
    text: TextSchema,
}

/// The note of a file, as it is written: a view of the note, so that writing it copies none of its paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct NoteViewSchema<'a> {
    /// The imports the file writes, in source order.
    imports: Vec<ImportViewSchema<'a>>,
}

/// One import, as it is written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct ImportViewSchema<'a> {
    /// What the path names.
    form: FormSchema,
    /// How many packages up it starts.
    level: LevelSchema,
    /// The path, without its leading dots.
    path: &'a str,
    /// Where the alias starts.
    start: OffsetSchema,
    /// Where it ends.
    end: OffsetSchema,
    /// The alias as the file writes it, its whitespace left out.
    text: &'a str,
}

impl<'a> From<&'a Note> for NoteViewSchema<'a> {
    fn from(note: &'a Note) -> Self {
        Self {
            imports: note
                .imports
                .iter()
                .map(|import| ImportViewSchema {
                    form: match import.form {
                        Form::Module => FormSchema::Module,
                        Form::Name => FormSchema::Name,
                        Form::Everything => FormSchema::Everything,
                    },
                    level: LevelSchema(import.level.0),
                    path: &import.path.0,
                    start: OffsetSchema(import.start.0),
                    end: OffsetSchema(import.end.0),
                    text: &import.text.0,
                })
                .collect(),
        }
    }
}

impl From<NoteSchema> for Note {
    fn from(schema: NoteSchema) -> Self {
        Self {
            imports: schema
                .imports
                .into_iter()
                .map(|import| ImportNote {
                    form: match import.form {
                        FormSchema::Module => Form::Module,
                        FormSchema::Name => Form::Name,
                        FormSchema::Everything => Form::Everything,
                    },
                    level: Level(import.level.0),
                    path: DottedPath(import.path.0.into()),
                    start: Offset(import.start.0),
                    end: Offset(import.end.0),
                    text: Spelling(import.text.0.into()),
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
