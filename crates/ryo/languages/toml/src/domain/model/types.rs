//! The types of the `model` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::keys::types::{ChildKey, KeyHash, Named};
use ryo::languages::{self, Position};
use std::collections::HashMap;

/// What a mapping of a document is, as the `mapping` relation writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MappingFact {
    /// The element that first opens it.
    pub id: Position,
    /// How it is written.
    pub kind: languages::MappingKind,
    /// The element of the entry or sequence element that holds it.
    pub owner: Position,
    /// Its place in a sequence of mappings, otherwise zero.
    pub ordinal: languages::Integer,
}

/// What an entry of a document is, as the `entry` relation writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EntryFact {
    /// The part of a key that opens it, or the last part of its key-value.
    pub occurrence: Position,
    /// What its value is.
    pub kind: languages::ValueKind,
    /// The element of the document or mapping that holds it.
    pub owner: Position,
    /// Its place among the entries of its owner.
    pub ordinal: languages::Integer,
    /// The element of its value; none when the key opens a mapping.
    pub value: Option<Position>,
    /// Whether its value could be read.
    pub readability: Readability,
}

/// What an element of a sequence is, as the `sequence_element` relation writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementFact {
    /// The element: a value of an array, or a member of an array of tables.
    pub occurrence: Position,
    /// What it is.
    pub kind: languages::ValueKind,
    /// The element of the entry or element that holds the sequence.
    pub owner: Position,
    /// Its place in the sequence.
    pub ordinal: languages::Integer,
}

/// Whether the value of an entry could be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Readability {
    /// It could.
    Readable,
    /// It is missing or is no TOML value: the library reads it as a string it refuses.
    Unreadable,
}

/// The mappings, entries and sequence elements of one file, worked out in one pass, each list in the order of its elements.
#[derive(Debug, Default)]
pub(crate) struct Model {
    /// Every mapping, by its element.
    pub mappings: Vec<MappingFact>,
    /// Every entry, by its occurrence.
    pub entries: Vec<EntryFact>,
    /// Every sequence element, by its occurrence.
    pub elements: Vec<ElementFact>,
    /// What each key names within its mapping.
    pub children: HashMap<ChildKey, Named>,
    /// How many entries each mapping has so far.
    pub counts: HashMap<Position, languages::Integer>,
    /// How the pass hashes a key's text.
    pub hash: KeyHash,
}

/// Whether a string is a value, or a part of a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stringness {
    /// A string value.
    Value,
    /// A part of a key, or no string.
    Other,
}
