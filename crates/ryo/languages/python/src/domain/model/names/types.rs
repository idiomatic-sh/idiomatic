//! The types of the model's `names` part: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::qualification::types::Version;
use ryo::languages::{FixedWord, NameText, Offset, Symbol};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};

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

/// What the walk writes of the model as it writes the file: the names it interned, the lexical part (each node read as it
/// opens, `lexical::opened`) and the code of each name token of the file's own tree, in id order (none when the file's bytes
/// were not text). The model copies the table and the lexical part; the engine reads the token codes.
#[derive(Clone, Debug, Default)]
pub(crate) struct WalkNames {
    /// Collect expression classes while the parser visits the source tree.
    pub shape_classes: bool,
    /// The table the walk interned into.
    pub names: NameTable,
    /// The lexical part, by place: the name each node carries among it.
    pub lexical: crate::domain::model::lexical::types::LexicalModel,
    /// The code of each name token of the file's own tree, in id order.
    pub of_token: Vec<NameCode>,
}

/// The word of each name of the file that a filler has written so far, by the name's code.
#[derive(Clone, Debug, Default)]
pub(crate) struct NameWords(pub Vec<Option<Symbol>>);

/// A fixed text of the vocabulary by where it stands in the program: two uses of one constant are one key, and looking one up
/// reads no text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TextPlace(pub NameText<'static>);

impl PartialEq for TextPlace {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.0, other.0.0)
    }
}

impl Eq for TextPlace {}

impl Hash for TextPlace {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0.0.as_ptr().addr());
        state.write_usize(self.0.0.len());
    }
}

/// The fixed names of the run as a worker's fillers write them: each by its handle, which is the same word in every file of the
/// run, so nothing here is forgotten with a file.
#[derive(Clone, Debug, Default)]
pub(crate) struct FixedWords {
    /// The language's fixed names, by place: what its vocabulary states for the run. A worker's walk is born with them; a view
    /// made without them holds none, and its fillers write every text as a word of the file.
    pub declared: Vec<NameText<'static>>,
    /// The handle of each text a filler has written so far; none for a text that is no fixed name of the run.
    pub held: HashMap<TextPlace, Option<FixedWord>, BuildHasherDefault<KeyHasher>>,
}

/// The text of a dotted name being written for one row.
#[derive(Clone, Debug, Default)]
pub(crate) struct DottedText(pub String);

/// The words the fillers have written: kept from one filler to the next.
#[derive(Clone, Debug, Default)]
pub(crate) struct FactWords {
    /// The words of the names of the file in hand: forgotten with the file.
    pub names: NameWords,
    /// The handles of the vocabulary's fixed texts: kept for the run.
    pub fixed: FixedWords,
    /// The dotted name a fill of one key writes: kept so that such a fill allocates none.
    pub path: DottedText,
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

    fn write_usize(&mut self, number: usize) {
        self.write_u64(u64::try_from(number).unwrap_or(u64::MAX));
    }

    fn write_u64(&mut self, number: u64) {
        self.0 = (self.0.rotate_left(5) ^ number).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}
