//! The types of the `stored_facts` concept: what a run keeps of the words and the text of a file. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::descriptions::types::RelationName;
use ryo_relations::domain::relations::types::{Missing, Relation};

/// Why a relation is read only in part, as the reader said it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartialCause(pub Box<str>);

/// The kind of the rows that could be missing from a relation read in part: empty when any could be.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartialKind(pub Box<str>);

/// A relation read only in part, as the store keeps it: the relation by its name, its cause, and the kind of the rows that could be
/// missing by its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredPartial {
    /// The name of the relation.
    pub relation_name: RelationName,
    /// Why it is read in part.
    pub cause: PartialCause,
    /// The name of the kind of the rows that could be missing; empty when any could be.
    pub lacking_kind: PartialKind,
    /// What the rows missing are about.
    pub at: PartialScope,
}

/// The number of the file a partly read relation is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartialAt(pub u32);

/// What the rows missing from a relation read in part are about: one file, or the repository as a whole when the reader does not
/// say which.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartialScope {
    /// This file.
    OneFile(PartialAt),
    /// The repository as a whole.
    Repository,
}

/// A relation a reader said it read only in part while it wrote the rows of a file: which, why, and which rows could be missing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadInPart {
    /// The relation.
    pub relation: Relation,
    /// Why, as the reader said it.
    pub cause: PartialCause,
    /// Which rows could be missing.
    pub lacking: Missing,
    /// What the rows missing are about: a file, or the project as a whole.
    pub at: PartialScope,
}

/// A bucket, numbered by the core: where words wait for the cross-file finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bucket(pub u32);

/// Words packed seven bits to a byte, the high bit saying another byte follows: a small number is one byte, the largest ten.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackedWords(pub Vec<u8>);

/// The words every worker added to a bucket, one worker after another, packed: packed words run together are the words of each
/// one after the other.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptWords(pub PackedWords);

/// How long the arena of a worker was before a file's text was put in it: where the words' offsets start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArenaBase(pub u32);

/// What a text of bytes comes to: sixteen bytes that are the same for the same bytes and, for bytes that differ, differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Digest(pub [u8; 16]);

/// What a language said when it failed on a file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadFailure(pub Box<str>);

/// What a language wrote of a file for the project to read, as bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoteBytes(pub Vec<u8>);

/// The place of a language among the ones linked, in the order `linked` lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageNumber(pub usize);

/// What the language that read a file wrote of it for the project: the language, by its place among the ones linked, and its
/// notes, which only that language reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileNotes {
    /// The language that wrote the notes.
    pub language: LanguageNumber,
    /// The notes.
    pub notes: NoteBytes,
}

/// The text a file's names made that is in no file, as bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArenaBytes(pub Vec<u8>);
