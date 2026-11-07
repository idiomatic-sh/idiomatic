//! The types of the `project` use case: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use super::errors::ProjectError;
use crate::domain::files::types::{FilePath, ManifestName};
use crate::domain::project_context::types::{
    EntryObservation, GivenProjectInventory, ObservedEntries, ProjectSearchScope,
};
use crate::domain::reasons::types::ToolMessage;
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::types::{Bytes, Rows};
use ryo_relations::domain::relations::types::{Relation, RowWriter};

/// The repository's manifests, read once per run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectRequest {
    /// The repository's root directory, where the language's own tools run.
    pub root: RootPath,
    /// Where the repository's files are: on the disk below the root, where a language may read what it is not given, or only the
    /// manifests and files it is given, when it reads nothing from the disk and the root names nothing.
    pub repository: Repository,
    /// Search roots declared by the host, in resolution order. An undeclared scope carries no absence proof.
    pub search_scope: ProjectSearchScope,
    /// Entries explicitly supplied for a Given project, separate from source texts.
    pub given_inventory: GivenProjectInventory,
    /// One engine observation of each demanded candidate entry, reused as the project's receipt.
    pub observed_entries: ObservedEntries,
    /// The manifests, each with its path and its text: both came from disk.
    pub manifests: Vec<ManifestFile>,
    /// Every file of the repository the run lists, by path, in the order of their places: a file's place is the number it has in the rows.
    pub files: Vec<FilePath>,
    /// What the language wrote of each of those files when it read it, in the order of `files`; empty for a file it wrote nothing of.
    pub notes: Vec<Bytes>,
    /// The relations the run asks for, so that a language links the files to one another only when a relation of that asks for it.
    pub wanted: Vec<Relation>,
}

/// Where the files of a repository are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repository {
    /// Below the root, on the disk.
    Disk,
    /// Only the manifests and the files given with the request.
    Given,
}

/// What reading the manifests of a project came to: what the readers read their rows from, and the manifests the language's tools refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved<D = ()> {
    /// The manifests the tools could not read, with why: a finding of the run, never a stop of it.
    pub refused: Vec<Refused>,
    /// The manifests the rows are about that the run does not list, by path: the rows hold each by the number it has after the last
    /// file the run lists, in the order of this list. A language whose rows name no manifest answers none.
    pub unlisted: Vec<FilePath>,
    /// The files the language read beyond the manifests the request handed it, by path from the root (`../` for one above it): the
    /// configurations its tools look for above and in the repository, and the manifests it read from the disk, and the files it looked
    /// for and found none of. The engine digests each and reads the project again only when one is gone or changed, or one it found
    /// none of is there. A language that reads only what it is handed answers none.
    pub consulted: Vec<FilePath>,
    /// The exact entry observations handed to the project reader; these also determine warm reuse.
    pub observed_entries: Vec<EntryObservation>,
    /// What the language worked out besides, for the readers of this run alone: the run does not keep it and the engine never compares it.
    /// A language that works out nothing has `()`.
    pub derived: D,
}

/// A manifest the language's tools could not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    /// The manifest's path, relative to the repository root.
    pub path: FilePath,
    /// What the tool said.
    pub reason: ToolMessage,
}

/// One manifest file of a project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestFile {
    /// The manifest's path, relative to the repository root.
    pub path: FilePath,
    /// Its text.
    pub text: ManifestText,
}

/// What a manifest reader reads its rows from: the request, and what the language's `resolve` worked out for it. Nothing
/// of it is kept for another run.
pub struct ManifestInput<'a, D = ()> {
    /// The manifests, the files and the notes of the run.
    pub request: &'a ProjectRequest,
    /// What the language worked out when it read the manifests.
    pub derived: &'a D,
}

impl<D> Clone for ManifestInput<'_, D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D> Copy for ManifestInput<'_, D> {}

impl<D> std::fmt::Debug for ManifestInput<'_, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(ManifestInput))
            .field(stringify!(request), &self.request)
            .finish_non_exhaustive()
    }
}

/// A relation filled from the manifests of a project or the links between its files, which ryo owns the shape of: the
/// relation of ryo's catalog it writes rows of, named by type, and how the language reads it. A language declares one
/// for each manifest relation it reads, with the macro, and defines no shape of its own.
///
/// Sealed against accidental impls: only the `reads!` macro implements the marker it needs, and a hand-written marker is a finding of `readers-in-items`.
pub trait ManifestItem: crate::domain::answers::types::declared::Manifest {
    /// The relation of ryo's catalog the item writes rows of.
    type Relation: RelationsItem;

    /// What the language works out besides when it reads the manifests, which the reader reads: the language's own.
    type Derived: Send + Sync + 'static;

    /// That the language fills the relation from a project.
    const READER: ManifestReader<Self::Relation, Self::Derived>;
}

/// How a language reads the relation `R` from the manifests of a project, once for the run.
#[derive(Debug)]
pub struct ManifestReader<R: RelationsItem, D = ()> {
    /// What it writes: the rows of its relation alone, and the names it makes in the symbols.
    pub read: fn(&ManifestInput<'_, D>, &mut Symbols, &mut dyn Rows<<R as RelationsItem>::Row>),
    /// The relation the rows are of.
    pub relation: std::marker::PhantomData<fn() -> R>,
}

impl<R: RelationsItem, D> Clone for ManifestReader<R, D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: RelationsItem, D> Copy for ManifestReader<R, D> {}

/// The repository's root directory, as the file system names it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootPath(pub Box<str>);

impl std::ops::Deref for RootPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for RootPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The text of a manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManifestText(pub Box<str>);

impl std::ops::Deref for ManifestText {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for ManifestText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What the engine asks of a language whatever it works out: the same reading as `ProjectUseCase`, with what the language derives
/// kept inside the call, so that languages of different types are held side by side. Every `ProjectUseCase` is one.
pub trait ProjectEngine: Send + Sync {
    /// Which existing operation owns a primary file; this does not read or validate the file.
    fn primary_role(
        &self,
        path: crate::domain::files::types::SourcePath<'_>,
    ) -> crate::domain::files::types::PrimaryFileRole;

    /// Appends to `entries` the candidate entry paths needed for requested project relations.
    fn project_entries(&self, request: &ProjectRequest, entries: &mut Vec<FilePath>);

    /// The names of the files the language reads as manifests.
    fn manifest_names(&self) -> &'static [ManifestName];

    /// Reads the manifests and writes the rows of the wanted manifest relations into `out`; answers the manifests the
    /// language refused.
    ///
    /// # Errors
    ///
    /// What the language's `resolve` answers.
    fn project(
        &self,
        request: &ProjectRequest,
        symbols: &mut Symbols,
        out: &mut dyn RowWriter,
    ) -> Result<Resolved, ProjectError>;

    /// Why the language does not read `relation` from the manifests, or nothing when it does or when the relation is not one of the
    /// manifests'.
    fn absence_of(&self, relation: Relation) -> Option<AbsenceCause>;
}
