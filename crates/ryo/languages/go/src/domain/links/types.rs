//! The types of the `links` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

use ryo::languages::{FileId, Offset};

use crate::domain::fact_notes::types::{ImportPath, Spelling};
use crate::domain::modules::types::{Located, Manifest, ManifestPath, Modules};

/// An import written in one file that names the package another file is part of, by the files' numbers: an import of a package of
/// several files is one link to each.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Link {
    /// The file the import is written in.
    pub source: FileId,
    /// A file of the package it names.
    pub target: FileId,
    /// Where the import's spec starts.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The spec as the file writes it, its whitespace left out.
    pub written: Spelling,
}

/// Why the imports of a file are not all linked.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Doubt {
    /// An import path under the file's module that names no directory of Go files of it, as written.
    Path(ImportPath),
    /// The `go.mod` of the file's module, which the tool refused.
    Module(ManifestPath),
}

/// A file whose imports are not all linked, and why.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UnlinkedFile {
    /// The file.
    pub file: FileId,
    /// Why.
    pub doubt: Doubt,
}

/// What linking the files of a repository came to, with the modules the manifests declared.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Linked {
    /// The links, by source file, then place, then target.
    pub dependencies: Vec<Link>,
    /// The files whose imports were all examined, in order.
    pub analyzed: Vec<FileId>,
    /// The files whose imports are not all linked, by file.
    pub unresolved: Vec<UnlinkedFile>,
    /// What the manifests declared.
    pub modules: Modules,
}

/// What a directory holds a manifest of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Root<'a> {
    /// A module, read.
    Module(&'a Manifest),
    /// A module whose `go.mod` the tool refused.
    Refused(&'a ManifestPath),
}

/// The module a file is in: the one whose directory is the nearest above it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner<'a> {
    /// A module, read.
    Module(&'a Manifest),
    /// A module whose `go.mod` the tool refused.
    Refused(&'a ManifestPath),
    /// None: no `go.mod` is above it.
    Nothing,
}

/// Where an import leads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImportOutcome<'a> {
    /// To the files of a package of the repository.
    Files(&'a [FileId]),
    /// Outside the module: the standard library or another module, which `package_dependency` answers.
    Outside,
    /// To a package of the module no directory of Go files is.
    Unresolved,
}

/// The repository the links are made over: the Go files of each directory but its tests, by number, and the directories that hold a
/// `go.mod`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Layout<'a> {
    /// The files of the package each directory is, by the directory's path.
    pub packages: BTreeMap<Located, Vec<FileId>>,
    /// The manifest of each directory that holds a `go.mod`, by the directory's path.
    pub roots: BTreeMap<Located, Root<'a>>,
}

/// The place of a file among the files the run lists, in the order of their numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileIndex(pub usize);

/// The rest of an import path after its module's path, without the separator: the directory under the module's it names; empty for
/// the module's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rest<'a>(pub &'a str);
