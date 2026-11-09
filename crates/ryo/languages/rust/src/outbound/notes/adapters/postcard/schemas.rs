//! What postcard writes and reads, which the adapter does not control, and its conversion from and into the language's own types: the
//! wire shape of a file's note, written from the note without making a copy of it, and read into one that borrows the bytes.

use std::borrow::Cow;

use serde::ser::SerializeStruct as _;
use serde::{Deserialize, Serialize, Serializer};

use super::constants::{
    AROUND, BOUND, CLEAN, DECLARED, DECLARED_FIELD, DECLARED_FIELDS, ENCLOSING, END, INLINE, NAME,
    NOTE, NOTE_FIELDS, PATH, SEGMENTS, START, STEP, STEP_FIELDS, TEST_ONLY, TEXT, WRITTEN,
    WRITTEN_FIELD, WRITTEN_FIELDS,
};
use super::types::Leftover;
use crate::domain::fact_notes::types::{
    DeclaredNote, Note, NoteFlag, NoteName, NoteOffset, NotePath, StepNote, WrittenNote,
};
use crate::outbound::notes::errors::NotesError;
use ryo::languages::Bytes;

/// A flag, as it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct FlagSchema(bool);

/// A text, as it is written.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(super) struct TextSchema(String);

/// A place in a file, in bytes, as it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct OffsetSchema(u32);

/// A name borrowed from the bytes it is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct NameSchema<'a>(&'a str);

/// The note of a file, as it is read: it borrows the bytes.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct NoteSchema<'a> {
    /// Whether the file parsed without errors, so its paths can be trusted.
    clean: FlagSchema,
    /// The `mod` declarations the file holds.
    #[serde(borrow)]
    declared: Vec<DeclaredSchema<'a>>,
    /// The paths the file writes, which linking resolves.
    #[serde(borrow)]
    written: Vec<WrittenSchema<'a>>,
}

/// An inline block a declaration is written inside, as it is read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct StepSchema<'a> {
    /// The inline module's name.
    #[serde(borrow)]
    name: NameSchema<'a>,
    /// The `#[path]` the inline module is given, when it has one.
    path: Option<TextSchema>,
}

/// A `mod` declaration, as it is read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct DeclaredSchema<'a> {
    /// The inline modules the declaration is nested in, outermost first.
    #[serde(borrow)]
    around: Vec<StepSchema<'a>>,
    /// The declared module's name.
    #[serde(borrow)]
    name: NameSchema<'a>,
    /// Whether the module's body is written in place rather than in a file of its own.
    inline: FlagSchema,
    /// The `#[path]` the declaration is given, when it has one.
    path: Option<TextSchema>,
    /// Whether the declaration is under `#[cfg(test)]`.
    test_only: FlagSchema,
}

/// A path a file writes, as it is read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct WrittenSchema<'a> {
    /// The path's segments, as written.
    #[serde(borrow)]
    segments: Vec<NameSchema<'a>>,
    /// The inline modules the path is written inside, outermost first.
    #[serde(borrow)]
    enclosing: Vec<NameSchema<'a>>,
    /// Where the path starts in the file.
    start: OffsetSchema,
    /// Where the path ends in the file.
    end: OffsetSchema,
    /// The path as the file writes it, its whitespace left out.
    #[serde(borrow)]
    text: NameSchema<'a>,
    /// The name a `use` leaf binds, when it binds one.
    #[serde(borrow)]
    bound: Option<NameSchema<'a>>,
}

impl<'a> From<NoteSchema<'a>> for Note<'a> {
    fn from(schema: NoteSchema<'a>) -> Self {
        let path = |path: Option<TextSchema>| path.map(|each| NotePath(each.0));
        let names = |names: Vec<NameSchema<'a>>| -> Vec<NoteName<'a>> {
            names
                .into_iter()
                .map(|name| NoteName(Cow::Borrowed(name.0)))
                .collect()
        };
        Self {
            clean: NoteFlag(schema.clean.0),
            declared: schema
                .declared
                .into_iter()
                .map(|each| DeclaredNote {
                    around: each
                        .around
                        .into_iter()
                        .map(|step| StepNote {
                            name: NoteName(Cow::Borrowed(step.name.0)),
                            path: path(step.path),
                        })
                        .collect(),
                    name: NoteName(Cow::Borrowed(each.name.0)),
                    inline: NoteFlag(each.inline.0),
                    path: path(each.path),
                    test_only: NoteFlag(each.test_only.0),
                })
                .collect(),
            written: schema
                .written
                .into_iter()
                .map(|each| WrittenNote {
                    segments: names(each.segments),
                    enclosing: names(each.enclosing),
                    start: NoteOffset(each.start.0),
                    end: NoteOffset(each.end.0),
                    text: NoteName(Cow::Borrowed(each.text.0)),
                    bound: each.bound.map(|name| NoteName(Cow::Borrowed(name.0))),
                })
                .collect(),
        }
    }
}

/// The note of a file, as it is written: a view of the note itself, so that writing it makes no copy of it.
pub(super) struct NoteViewSchema<'v, 'n>(&'v Note<'n>);

/// One declaration, as it is written.
struct DeclaredViewSchema<'v, 'n>(&'v DeclaredNote<'n>);

/// One inline block, as it is written.
struct StepViewSchema<'v, 'n>(&'v StepNote<'n>);

/// One written path, as it is written.
struct WrittenViewSchema<'v, 'n>(&'v WrittenNote<'n>);

/// A list, as it is written: each of its items through `view`.
struct ListSchema<'v, T, V>(&'v [T], fn(&'v T) -> V);

/// The text of a name, as it is written.
fn text_of<'v>(name: &'v NoteName<'_>) -> &'v str {
    &name.0
}

impl Serialize for NoteViewSchema<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut note = serializer.serialize_struct(NOTE, NOTE_FIELDS)?;
        note.serialize_field(CLEAN, &self.0.clean.0)?;
        note.serialize_field(
            DECLARED_FIELD,
            &ListSchema(&self.0.declared, DeclaredViewSchema),
        )?;
        note.serialize_field(
            WRITTEN_FIELD,
            &ListSchema(&self.0.written, WrittenViewSchema),
        )?;
        note.end()
    }
}

impl Serialize for DeclaredViewSchema<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut declared = serializer.serialize_struct(DECLARED, DECLARED_FIELDS)?;
        declared.serialize_field(AROUND, &ListSchema(&self.0.around, StepViewSchema))?;
        declared.serialize_field(NAME, &*self.0.name.0)?;
        declared.serialize_field(INLINE, &self.0.inline.0)?;
        declared.serialize_field(PATH, &self.0.path.as_ref().map(|each| &each.0))?;
        declared.serialize_field(TEST_ONLY, &self.0.test_only.0)?;
        declared.end()
    }
}

impl Serialize for StepViewSchema<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut step = serializer.serialize_struct(STEP, STEP_FIELDS)?;
        step.serialize_field(NAME, &*self.0.name.0)?;
        step.serialize_field(PATH, &self.0.path.as_ref().map(|each| &each.0))?;
        step.end()
    }
}

impl Serialize for WrittenViewSchema<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut written = serializer.serialize_struct(WRITTEN, WRITTEN_FIELDS)?;
        written.serialize_field(SEGMENTS, &ListSchema(&self.0.segments, text_of))?;
        written.serialize_field(ENCLOSING, &ListSchema(&self.0.enclosing, text_of))?;
        written.serialize_field(START, &self.0.start.0)?;
        written.serialize_field(END, &self.0.end.0)?;
        written.serialize_field(TEXT, &*self.0.text.0)?;
        written.serialize_field(BOUND, &self.0.bound.as_ref().map(|name| &*name.0))?;
        written.end()
    }
}

impl<T, V: Serialize> Serialize for ListSchema<'_, T, V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.0.iter().map(self.1))
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
pub(super) fn write_as(note: &Note<'_>, out: &mut Bytes) {
    let mut buffer = std::mem::take(&mut out.0);
    buffer.clear();
    out.0 = postcard::to_extend(&NoteViewSchema(note), buffer).unwrap_or_default();
}

/// The note `bytes` hold, when they hold exactly one.
pub(super) fn note_from(bytes: &Bytes) -> Result<Note<'_>, NotesError> {
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
