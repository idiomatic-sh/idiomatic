//! The types of the model's `names` part: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::qualification::types::Version;
use ryo::languages::Offset;
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
