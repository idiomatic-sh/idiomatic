//! The types of the `file` concept: a file of the repository by its path, and a file nothing read, with why.

/// A path relative to the root of the repository.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelativePath(pub Box<str>);

/// Why a language could not read a file, as it said.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadFailure(pub Box<str>);

/// Why a file was not read.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Unreadable {
    /// No linked language reads files like it.
    NoLanguage,
    /// A language that reads files like it failed on it.
    Failed(ReadFailure),
}

/// A file nothing read.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Unread {
    /// The file's path, relative to the root.
    pub path: RelativePath,
    /// Why it was not read.
    pub reason: Unreadable,
}
