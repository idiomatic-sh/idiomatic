//! The types of the `corpus_copies` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

use ryo_values::domain::files::types::FileBytes;

/// The name of a corpus item, which is the name of its relation.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ItemName(pub String);

/// How the files of the language end, without the dot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Extension(pub String);

/// The input of each item of a corpus, by the item's name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CorpusInputs(pub BTreeMap<ItemName, FileBytes>);

/// How many copies of every input a list holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyCount(pub u32);

/// Which copy a file is, counted from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyNumber(pub u32);

/// How many times the bytes of an input are written one after the other in a copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repeat(pub usize);

/// How many digits the number of a copy is written with in its path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyDigits(pub usize);

/// What the path of a file is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyNaming {
    /// `<item>/input.<extension>`, the one file of an item.
    Input,
    /// `<item>/copy_<number>.<extension>`, the number padded with zeros.
    Copy {
        /// The width of the number.
        digits: CopyDigits,
    },
}

/// Which files a list holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListShape {
    /// What the paths are called.
    pub naming: CopyNaming,
    /// How many copies of every input.
    pub copies: CopyCount,
    /// How many times each input's bytes are repeated in a copy.
    pub repeat: Repeat,
}
