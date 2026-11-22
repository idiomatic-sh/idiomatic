//! The types of the `bytes` part: the literals a rule set's needs ask the file's texts to hold, and a file's answers.

use crate::domain::code::types::WordSlot;

/// The literals the needs ask for, numbered as they are met, found once a worker: each byte term's alternatives (one of
/// them must be in the file), and a searcher for each literal longer than a byte.
#[derive(Debug, Default)]
pub(crate) struct ByteLiterals {
    /// The number of each literal.
    pub(crate) numbers: hashbrown::HashMap<Box<[u8]>, u32>,
    /// The literals, by number: a byte is read off the file's byte set, a longer one searched.
    pub(crate) literals: Vec<ByteSearch>,
    /// Each byte term's place among `wants`, by its regular expression and constant.
    pub(crate) terms: hashbrown::HashMap<(WordSlot, WordSlot), u32>,
    /// What each byte term asks: the numbers of its alternatives; none when it asks nothing of the file.
    pub(crate) wants: Vec<Option<Box<[u32]>>>,
}

/// How one literal is looked for in a file.
#[derive(Debug)]
pub(crate) enum ByteSearch {
    /// A single byte: the file's byte set says.
    Byte(u8),
    /// A longer literal: searched, stopping at its first occurrence.
    Text(memchr::memmem::Finder<'static>),
}

/// What a file's bytes hold of the literals asked, as bits by the literals' numbers.
#[derive(Debug, Default)]
pub(crate) struct ByteFile {
    /// How many of the literals the bits answer: none at the file's start.
    pub(crate) known: usize,
    /// The bytes the file holds, a bit a byte value: read once a file, with the first literal asked.
    pub(crate) set: [u64; 4],
    /// The literals the file holds.
    pub(crate) held: Vec<u64>,
}
