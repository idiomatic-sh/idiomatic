//! The types of the `parse` use case: plain data, public fields and derives, with no constructor
//! and no inherent impl.

use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::{Count, KindBits};
use ryo_relations::domain::terms::types::UnknownName;

/// The canonical, immutable identity of parser settings that can change the tree or diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParseConfigurationIdentity<'a>(pub &'a str);

/// One file to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseRequest<'a> {
    /// The file's path, relative to the repository root: text that came from disk.
    pub path: SourcePath<'a>,
    /// The number the run gave the file.
    pub file: FileId,
    /// The file's bytes.
    pub bytes: ByteSlice<'a>,
    /// Whether the language writes its notes of the file in the table, for the project to read: a language that has none ignores it.
    pub notes: NoteRequest,
    /// Whether the roles of the elements are asked for.
    pub fields: FieldRequest,
}

/// A file a language reads, for the contract to read.
#[derive(Clone, Copy, Debug)]
pub struct SourceFile {
    /// The path the file has in a repository.
    pub path: SourcePath<'static>,
    /// The file's bytes.
    pub bytes: ByteSlice<'static>,
}

/// How a language shows one kind of its trees: the file whose tree holds it, or why no relation reads it. Every kind has one, as the
/// match that gives it has no wildcard arm, so a kind the parser gains is a compile error until it is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sample {
    /// A file whose tree holds a node of the kind.
    Node(SampleText),
    /// A file whose tree holds a token of the kind.
    Token(SampleText),
    /// No relation models the kind, for this reason; the parser may never give it.
    Unread(Why),
}

/// One kind of a language's trees with how the language shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindSample {
    /// The kind's name in the language's code (`Fn`), the one to write in a reader's `kinds:`.
    pub kind: KindName,
    /// The number the element table holds for the kind.
    pub bits: KindBits,
    /// How the language shows it.
    pub sample: Sample,
}

/// The source text of a file that shows a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleText(pub &'static str);

impl std::ops::Deref for SampleText {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Why no relation models a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Why(pub &'static str);

impl std::ops::Deref for Why {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// The name of a kind in the language's code (`Fn`): the one to write in a reader's `kinds:`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KindName(pub Box<str>);

impl std::ops::Deref for KindName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for KindName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether a language reads a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recognition {
    /// It does.
    Reads,
    /// It does not: the file is none of its.
    Declines,
}

/// Whether a request asks the language for the notes of a file: bytes only it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteRequest {
    /// It does.
    Wanted,
    /// It does not.
    Unwanted,
}

/// Whether a request asks the language for the role of every element under its parent: the `field` of the edges of the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldRequest {
    /// It does.
    Wanted,
    /// It does not: no query of the run reads the column.
    Unwanted,
}

/// The name of a directory a language never reads, at any depth (`node_modules`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkippedDirectory(pub &'static str);

/// The names of the kinds a language's trees have, each once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindNames(pub Vec<ryo_relations::domain::terms::types::NameText<'static>>);

/// What the engine asks of a language besides its walk, erased: which files it reads, what it skips, the kinds its trees have, and what
/// it promises. Its trees and rows come from its walk; there is no old reader to ask.
pub trait ParseEngine: Send + Sync {
    /// Necessary spelling guarantees for this implementation's syntax and fact columns.
    fn source_words(&self) -> Option<super::named::SourceWords> {
        None
    }

    /// Provider-owned file relations beyond the shared catalog, and the numbered visit that lends the file: required of every language.
    fn file_relations(&self) -> &dyn super::named::FileRelations;

    /// Every optional promise of the language, as it lists them in `ParseUseCase::capabilities`.
    fn capabilities(&self) -> Vec<ryo_relations::domain::catalog::types::OptionalCapability>;

    /// Canonical parser settings included in persisted-result cache identity.
    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_>;

    /// Whether the language reads the file in `request`.
    fn recognition(&self, request: &ParseRequest<'_>) -> Recognition;

    /// The names of the directories the language never reads, at any depth.
    fn skips(&self) -> &'static [SkippedDirectory];

    /// How many kinds of node and token the language's trees have.
    fn kind_count(&self) -> Count;

    /// The names of the kinds the language's trees have.
    fn kind_names(&self) -> KindNames;

    /// Field codes whose meaning stays fixed across files. Providers with file-local dictionaries leave this empty.
    fn stable_fields(&self) -> crate::domain::fillers::types::FieldNames {
        Default::default()
    }

    /// The reasons the language declares for a value it cannot resolve statically.
    fn unknown_reasons(&self) -> &'static [UnknownName];
}
