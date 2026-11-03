//! The types of the `links` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::{FileId, Offset};

use crate::domain::fact_notes::types::Spelling;
use crate::domain::packages::types::Packages;

/// An import written in one file that names a module another file is, by the files' numbers.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Link {
    /// The file the import is written in.
    pub source: FileId,
    /// The file of the module it names.
    pub target: FileId,
    /// Where the import's alias starts.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The alias as the file writes it, its whitespace left out.
    pub written: Spelling,
}

/// An import as it is written, its leading dots kept: `..m.x`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct WrittenImport(pub Box<str>);

/// An import that names a module of the repository no file is: a relative import, or an absolute one whose first part is a module or a
/// package of the repository.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UnlinkedImport {
    /// The file it is written in.
    pub file: FileId,
    /// The import, as written.
    pub written: WrittenImport,
}

/// What linking the files of a repository came to, with the packages the manifests declared.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Linked {
    /// Requested metadata diagnostics, independent of dependency projection.
    pub manifest_validation: crate::domain::manifest_validation::types::ManifestValidations,
    /// Optional project module identities and explicit context incompleteness.
    pub module_context: crate::domain::module_context::types::ModuleContexts,
    /// The links, by source file, then place, then target.
    pub dependencies: Vec<Link>,
    /// The Python files whose imports were all examined, in order.
    pub analyzed: Vec<FileId>,
    /// The imports that name a module of the repository no file is, by file.
    pub unresolved: Vec<UnlinkedImport>,
    /// What the manifests declared.
    pub packages: Packages,
}

/// What a dotted path names under a directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Found {
    /// The file of a module: its number.
    File(FileId),
    /// A directory of Python files without `__init__.py`: a namespace package, which no file is.
    Namespace,
    /// Nothing.
    Nothing,
}

/// Where an import leads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImportOutcome {
    /// To a file of the repository.
    File(FileId),
    /// To a namespace package of the repository, which no file is.
    Namespace,
    /// Outside the repository: no module or package of it has the import's first part as its name.
    Outside,
    /// To a module of the repository no file is.
    Unresolved,
}

/// The files of a repository the links are made over: their paths, in the order of their numbers and of their paths, and every
/// directory that holds a Python file at any depth.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Listing<'a> {
    /// The paths of the files.
    pub files: &'a [ryo::languages::FilePath],
    /// The directories that hold a Python file, at any depth.
    pub directories: &'a std::collections::BTreeSet<crate::domain::patterns::types::Directory<'a>>,
}

/// The path of a file, relative to the root of the repository: one the run lists, or one a module may be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FilePlace<'a>(pub &'a str);

/// A dotted path without leading dots, or the part of one before its last name; empty for none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Dotted<'a>(pub &'a str);

/// The place of a file among the files the run lists, in the order of their numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileIndex(pub usize);
