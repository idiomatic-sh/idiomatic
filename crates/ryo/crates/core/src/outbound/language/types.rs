//! The types of the `language` port: plain data, public fields and derives, with no constructor
//! and no inherent impl.

use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_relations::domain::terms::types::UnknownName;

use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::reasons::types::ToolMessage;
use ryo_language::inbound::use_cases::project::types::{ManifestFile, RootPath};
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::files::types::{CorpusInput, FileBytes};
use ryo_values::domain::stored_facts::types::{FileNotes, LanguageNumber, ReadFailure};

/// A linked language as a listing names it: its name, then its build.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkedName(pub Box<str>);

/// What the language that read each file wrote of it, in the order of the files: nothing for a file no language noted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectNotes(pub Vec<Option<FileNotes>>);

/// Whether a language writes its notes of a file for the project to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteWriting {
    /// It writes them.
    Wanted,
    /// It does not.
    Unwanted,
}

/// Whether a linked language reads a file as a manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestReading {
    /// One does.
    Read,
    /// None does.
    NotRead,
}

/// The reasons the linked languages declare for a value they cannot resolve statically, every language's together: what each symbol of
/// the block of reasons is called.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnknownReasons(pub Vec<UnknownName>);

/// Which build of a language this is.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageBuild(pub Box<str>);

/// One file to read, and the relations wanted of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileToRead<'a> {
    /// A linked provider explicitly selected by the request edge.
    pub source_language: Option<&'a LanguageName>,
    /// The path, relative to the root: text that came from disk.
    pub path: &'a FilePath,
    /// The number the run gave the file.
    pub file: FileId,
    /// The bytes.
    pub bytes: &'a FileBytes,
    /// The relations the query reads.
    pub wanted: &'a [Relation],
    /// Whether the language writes its notes of the file for the project to read.
    pub notes: NoteWriting,
    /// Whether the language gives the elements their roles.
    pub roles: RoleWriting,
}

/// What reading one file came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileReading {
    /// The language at this place among the ones linked read it and wrote its rows.
    Read(LanguageNumber),
    /// A provider classifies this input as project/configuration data: its project reader owns semantics, it has no source tree, and it is not left unread.
    Manifest,
    /// No linked language reads this file.
    Unsupported,
    /// A language reads files like this one, and failed on this one.
    Failed(ReadFailure),
}

/// Whether a language reads a relation, and when it does not, why.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Absence {
    /// The language reads the relation.
    #[default]
    Reads,
    /// The language does not read it, for this reason.
    Because(AbsenceCause),
}

/// What one linked language answers for a relation: whether it reads it, and when it does not, why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationAnswer {
    /// The language's name.
    pub language: LanguageName,
    /// Whether the language reads the relation, and when it does not, why.
    pub absence: Absence,
    /// The reasons the language declares for a value it cannot resolve statically, in its own order.
    pub unknowns: &'static [UnknownName],
}

/// What the languages are given to write the rows the project gives: the manifests, and the files of the repository.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectInput<'a> {
    /// The repository's root directory.
    pub root: &'a RootPath,
    /// Where its files are: on the disk below the root, or only those given.
    pub repository: ryo_language::inbound::use_cases::project::types::Repository,
    /// Ordered host-declared project search roots.
    pub search_scope: &'a ryo_language::domain::project_context::types::ProjectSearchScope,
    /// Host inventory authority for Given requests.
    pub given_inventory: &'a ryo_language::domain::project_context::types::GivenProjectInventory,
    /// Entry states acquired once by the engine for the demanded candidates.
    pub observed_entries: &'a [ryo_language::domain::project_context::types::EntryObservation],
    /// The manifests of the repository.
    pub manifests: &'a [ManifestFile],
    /// Every file the run lists, by path, in the order of their places.
    pub files: &'a [CorpusInput],
    /// What the language that read each file wrote of it, in the order of `files`.
    pub notes: &'a ProjectNotes,
    /// The relations the run asks for.
    pub wanted: &'a [Relation],
}

/// A manifest a language's tools refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefusedManifest {
    /// The language.
    pub language: LanguageName,
    /// The manifest's path, relative to the root.
    pub path: FilePath,
    /// What the tool said.
    pub reason: ToolMessage,
}

/// What reading the manifests of the repository with every linked language came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectAnswer {
    /// The manifests the languages' tools refused: findings of the run.
    pub refused: Vec<RefusedManifest>,
    /// The manifests the rows are about that the run does not list: the rows hold each by the number it has after the last file the
    /// run lists, in the order of this list.
    pub unlisted: Vec<FilePath>,
    /// The files the languages read beyond the manifests the run handed them, and the manifests the rows are about that the run does
    /// not list, by path from the root (`../` climbs above it), in the order of their paths, each once.
    pub consulted: Vec<FilePath>,
    /// The exact candidate observations the engine handed to the readers.
    pub observed_entries: Vec<ryo_language::domain::project_context::types::EntryObservation>,
}

/// Whether a language gives the elements of a file their roles under their parents, for the queries that read them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoleWriting {
    /// It does: a query of the run reads the `field` column of `syntax-child`.
    Wanted,
    /// It does not.
    Unwanted,
}

/// The relations ryo owns, in the order the catalog lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationList(pub Vec<Relation>);

/// The names of the languages linked, in the order they were linked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkedNames(pub Vec<LinkedName>);

/// What every linked language answers for a relation, in the order they were linked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answers(pub Vec<RelationAnswer>);

/// The parts of one language an edge hands over: its name, its build, and the two use cases it implements. One value for each
/// language, so no part can be missing or out of step with another.
// ignore: idiomatic.arch/port/plain-port-types - the parts of one language an edge hands the adapter: its two use cases are shared engines behind `dyn` that nothing compares, copies or prints; the adapter takes them whole, and declaring them in factory.rs would make the adapter name its factory (factory-calls)
pub struct LanguageHandles {
    /// What the language is called, in listings.
    pub name: LanguageName,
    /// Which build of the language this is.
    pub build: LanguageBuild,
    /// Reads one file.
    pub parse: std::sync::Arc<dyn ryo_language::inbound::use_cases::parse::types::ParseEngine>,
    /// Reads the repository's manifests.
    pub project:
        std::sync::Arc<dyn ryo_language::inbound::use_cases::project::types::ProjectEngine>,
}

impl std::fmt::Debug for LanguageHandles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
