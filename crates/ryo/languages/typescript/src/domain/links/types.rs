//! The types of the `links` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::{FileId, Offset};

use crate::domain::fact_notes::types::{ImportSpelling, Specifier};
use crate::domain::packages::types::Packages;

/// An import written in one file that names another file of the repository, by the files' numbers.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TypeScriptImportLink {
    /// The file the import is written in.
    pub source: FileId,
    /// The file it names.
    pub target: FileId,
    /// Where the import starts.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The import as the file writes it, its whitespace left out.
    pub written: ImportSpelling,
}

/// An import that names a file of the repository no file is: a relative specifier, a `#` import of its package, or one a pattern of
/// the configuration's `paths` maps.
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
    pub dependencies: Vec<TypeScriptImportLink>,
    /// The files whose imports were all examined, in order.
    pub analyzed: Vec<FileId>,
    /// The imports that name a file of the repository no file is, by file.
    pub unresolved: Vec<Unresolved>,
    /// What the manifests declared.
    pub packages: Packages,
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

/// The text the `*` of a pattern of `paths` stands for in a specifier; empty for a pattern without one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Matched<'a>(pub &'a str);

/// The `paths` and the `baseUrl` that hold for the files of a project configuration.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Options<'a> {
    /// The aliases of `paths`.
    pub aliases: &'a [crate::domain::packages::types::PathAlias],
    /// The `baseUrl`, from the repository root.
    pub base: Option<&'a crate::domain::packages::types::ManifestPath>,
}
