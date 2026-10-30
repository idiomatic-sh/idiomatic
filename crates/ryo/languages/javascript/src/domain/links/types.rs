//! The types of the `links` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::{FileId, Offset};

use crate::domain::fact_notes::types::{Specifier, Spelling};

/// An import written in one file that names another file of the repository, by the files' numbers.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Link {
    /// The file the import is written in.
    pub source: FileId,
    /// The file it names.
    pub target: FileId,
    /// Where the import starts.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The import as the file writes it, its whitespace left out.
    pub written: Spelling,
}

/// An import that names a file of the repository no file is: a relative specifier, or a `#` import of its package.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Unresolved {
    /// The file it is written in.
    pub file: FileId,
    /// The specifier, as written.
    pub specifier: Specifier,
}

/// What linking the files of a repository came to, with the packages the manifests declared.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Linked {
    /// The links, by source file, then place, then target.
    pub dependencies: Vec<Link>,
    /// The files whose imports were all examined, in order.
    pub analyzed: Vec<FileId>,
    /// The imports that name a file of the repository no file is, by file.
    pub unresolved: Vec<Unresolved>,
    /// What the manifests declared.
    pub packages: crate::domain::packages::types::Packages,
}

/// Where an import leads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImportOutcome {
    /// To a file of the repository.
    File(FileId),
    /// Outside the repository, or to a package of it by its name, which `package_dependency` answers.
    Outside,
    /// To a file of the repository no file is.
    Unresolved,
}

/// The place of a file among the files the run lists, in the order of their numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileIndex(pub usize);

/// Why the relation is read in part: an import that names a file of the repository no file is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Cause(pub Box<str>);

/// A path a specifier may name, from the repository root, its `.` and `..` parts gone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Candidate(pub Box<str>);
