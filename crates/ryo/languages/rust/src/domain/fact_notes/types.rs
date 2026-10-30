//! The types of the `fact_notes` concept: plain data, public fields and derives, with no constructor and no inherent impl. The notes of a
//! file are these types; the port `notes` writes them as bytes and reads them back.

use std::borrow::Cow;

/// Whether the first segment of a path names a module of the repository.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Naming {
    /// It does.
    Known,
    /// It names none.
    Unknown,
}

/// A name or a path in the notes: borrowed from the file when the parse writes it, and the reader's own when it is read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NoteName<'a>(pub Cow<'a, str>);

/// A path the notes hold that is made after the file was read (the value of a `#[path = "…"]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NotePath(pub String);

/// Whether something is so, in the notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NoteFlag(pub bool);

/// A place in the text of the file, in bytes, in the notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NoteOffset(pub u32);

/// The notes of a file: what it says of its modules.
#[derive(Clone, Debug, PartialEq, Eq)]
// ignore: idiomatic.arch/mirrored-types - the notes of a file as the notes port holds them, borrowed from the encoded bytes; its look-alike is the module tree's owned `Facts`, which the notes are converted to once
pub(crate) struct Note<'a> {
    /// Whether the file parses without errors.
    pub clean: NoteFlag,
    /// The modules it declares, in source order.
    pub declared: Vec<DeclaredNote<'a>>,
    /// The paths it writes, in source order.
    pub written: Vec<WrittenNote<'a>>,
}

/// An inline module block a declaration is written inside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StepNote<'a> {
    /// The module's name.
    pub name: NoteName<'a>,
    /// The value of its `#[path = "…"]`, when it has one.
    pub path: Option<NotePath>,
}

/// One `mod` declaration of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeclaredNote<'a> {
    /// The inline blocks it is written inside, outermost first.
    pub around: Vec<StepNote<'a>>,
    /// The module's name.
    pub name: NoteName<'a>,
    /// Whether it is `mod name { … }`.
    pub inline: NoteFlag,
    /// The value of its `#[path = "…"]`, when it has one.
    pub path: Option<NotePath>,
}

/// A path a file writes.
#[derive(Clone, Debug, PartialEq, Eq)]
// ignore: idiomatic.arch/mirrored-types - a written path as the notes port holds it, borrowed from the encoded bytes; its look-alike is the module tree's owned `Written`, which it is converted to once
pub(crate) struct WrittenNote<'a> {
    /// The segments.
    pub segments: Vec<NoteName<'a>>,
    /// The names of the inline modules around it, outermost first.
    pub enclosing: Vec<NoteName<'a>>,
    /// Where it starts, in bytes.
    pub start: NoteOffset,
    /// Where it ends, in bytes.
    pub end: NoteOffset,
    /// The path as the file writes it, its whitespace left out: the text from `start` to `end`.
    pub text: NoteName<'a>,
}
