//! The types of the `globs` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

/// A directory holding a path, as text: the root is empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Holder<'a>(pub &'a str);

/// A directory of the governed files, owned: what the counts of a plan are kept by.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HolderPath(pub Box<str>);

impl std::borrow::Borrow<str> for HolderPath {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// How many files a directory holds, at any depth.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileCount(pub usize);

/// How many governed files each directory holds, at any depth: what a set of reached files is compressed against.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileCounts(pub BTreeMap<HolderPath, FileCount>);

/// Whether a directory is within another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Within {
    /// It is the other, or beneath it.
    Inside,
    /// It is not.
    Outside,
}
