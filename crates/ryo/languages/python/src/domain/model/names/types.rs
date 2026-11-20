//! The types of the model's `names` part: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::qualification::types::Version;
use ryo::languages::{NameText, Offset, Symbol};
use std::hash::Hasher;

/// A name of the file: equal texts have equal codes, in the order the names are first met.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NameCode(pub u32);

impl From<NameCode> for usize {
    fn from(code: NameCode) -> Self {
        Self::try_from(code.0).unwrap_or(Self::MAX)
    }
}

/// The hash of a name's text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NameHash(pub u64);

/// One slot of the name table: the place of an entry plus one, or zero when the slot is free.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NameSlot(pub u32);

/// The texts of every name of the file, one after another.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct NameTexts(pub String);

/// One name of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NameEntry {
    /// Where its text starts in the table's texts.
    pub start: Offset,
    /// The byte after its text.
    pub end: Offset,
    /// The hash of its text.
    pub hash: NameHash,
    /// The language version whose builtins first have the name, when they have it.
    pub builtin: Option<Version>,
}

/// The names of one file: cleared between files, never reallocated.
#[derive(Clone, Debug, Default)]
pub(crate) struct NameTable {
    /// The texts.
    pub texts: NameTexts,
    /// One entry per name, by code.
    pub entries: Vec<NameEntry>,
    /// The open-addressed slots, a power of two of them.
    pub slots: Vec<NameSlot>,
}

/// The word of each name of the file that a filler has written so far, by the name's code.
#[derive(Clone, Debug, Default)]
pub(crate) struct NameWords(pub Vec<Option<Symbol>>);

/// One fixed text of the facts' vocabulary with the word it has in the file's words.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HeldWord {
    /// The text: a constant of the vocabulary.
    pub text: NameText<'static>,
    /// Its word.
    pub word: Symbol,
}

/// The fixed texts written so far for the file in hand: a handful, each interned once and then written from here.
#[derive(Clone, Debug, Default)]
pub(crate) struct HeldWords(pub Vec<HeldWord>);

/// The words the fillers have written for the file in hand: kept from one filler to the next, forgotten with the file.
#[derive(Clone, Debug, Default)]
pub(crate) struct FactWords {
    /// The words of the file's names.
    pub names: NameWords,
    /// The words of the vocabulary's fixed texts.
    pub fixed: HeldWords,
}

/// The hasher of the model's keyed tables: the keys are a few small numbers, mixed one multiply each.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct KeyHasher(pub u64);

impl Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write_u64(u64::from(*byte));
        }
    }

    fn write_u32(&mut self, number: u32) {
        self.write_u64(u64::from(number));
    }

    fn write_u64(&mut self, number: u64) {
        self.0 = (self.0.rotate_left(5) ^ number).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}
