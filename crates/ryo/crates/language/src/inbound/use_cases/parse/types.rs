//! The types of the `parse` use case: plain data, public fields and derives, with no constructor
//! and no inherent impl.

use super::errors::ParseError;
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use crate::domain::semantics::types::SemanticSlot;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::{Count, KindBits, Relation, RowWriter};
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

/// What the engine asks of a language: the same reading as `ParseUseCase`, behind an interface alone and with what the language works
/// out of each file kept inside the call, so that languages of different models are held side by side. Every `ParseUseCase` is one.
pub trait ParseEngine: Send + Sync {
    /// Explicit optional promises, preserved across the erased provider boundary.
    fn capabilities(
        &self,
    ) -> &'static [ryo_relations::domain::relations::types::OptionalCapability];

    /// Canonical parser settings included in persisted-result cache identity.
    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_>;

    /// Begin this provider's owned semantic session for one built file.
    fn begin_semantics(&self, request: &ParseRequest<'_>, tree: &Elements, slot: &mut SemanticSlot);

    /// Whether the language reads the file in `request`.
    fn recognition(&self, request: &ParseRequest<'_>) -> Recognition;

    /// The names of the directories the language never reads, at any depth.
    fn skips(&self) -> &'static [SkippedDirectory];

    /// How many kinds of node and token the language's trees have.
    fn kind_count(&self) -> Count;

    /// The names of the kinds the language's trees have.
    fn kind_names(&self) -> KindNames;

    /// The reasons the language declares for a value it cannot resolve statically.
    fn unknown_reasons(&self) -> &'static [UnknownName];

    /// The kinds the readers of the language look at.
    fn reader_kinds(&self) -> Vec<KindBits>;

    /// Builds the tree of the file in `table`.
    ///
    /// # Errors
    ///
    /// What the language's `read_tree` answers.
    fn build_file(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
    ) -> Result<(), ParseError>;

    /// Does only what the language's library does for the file the request names, through a builder that builds nothing.
    ///
    /// # Errors
    ///
    /// What the library says of a file it refuses.
    fn baseline_file(&self, request: &ParseRequest<'_>) -> Result<(), ParseError>;

    /// Reads one file: the language builds its tree, and the rows of the wanted relations are written into `out`.
    ///
    /// # Errors
    ///
    /// What the language's `read_tree` answers.
    fn read_file(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    ) -> Result<(), ParseError>;

    /// Reads a file explicitly assigned to this provider, without rewriting its path.
    /// Providers must opt in; unsupported selection is never normal path fallback.
    ///
    /// # Errors
    /// `Unsupported` unless the provider supports explicit selection.
    fn read_file_explicit(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    ) -> Result<(), ParseError>;

    /// Writes the rows of the wanted relations for the file whose table is `elements`.
    fn walk_file(
        &self,
        elements: &mut Elements,
        symbols: &mut Symbols,
        source: ByteSlice<'_>,
        file: FileId,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    );

    /// Why the language does not read `relation` from a tree, or nothing when it does or when the relation is not one of the trees'.
    fn absence_of(&self, relation: Relation) -> Option<AbsenceCause>;
}

/// The catalog defines the exact set of reader slots held by one parse invocation.
macro_rules! reader_slots {
    (syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
     manifest { $( $manifest:tt )* } engine { $( $engine:tt )* }) => {
        /// The reader of one syntax relation, whichever it is.
        #[derive(Clone, Copy)]
        pub(super) enum Slot<M: 'static> {
            $( $variant(crate::domain::answers::types::Reader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, M>), )*
        }

        /// Which elements the reader in `slot` looks at.
        pub(super) fn reads_of<M>(slot: &Slot<M>) -> crate::domain::answers::types::Reads {
            match slot {
                $( Slot::$variant(reader) => reader.reads, )*
            }
        }

        /// Gives the element at `position` to the reader in `slot`, which writes its relation's rows into `out`.
        pub(super) fn visit<M>(
            slot: &Slot<M>,
            tree: &crate::domain::elements::types::Tree<'_>,
            model: &M,
            symbols: &mut crate::domain::symbols::types::Symbols,
            position: ryo_relations::domain::relations::types::Position,
            out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
        ) where M: 'static {
            match slot {
                $( Slot::$variant(reader) => reader.visit.visit(tree, model, symbols, position, out), )*
            }
        }

        /// Whether the reader in `slot` also looks at the elements the language added for reading.
        pub(super) fn expands<M>(slot: &Slot<M>) -> crate::domain::answers::types::Expansions where M: 'static {
            match slot {
                $( Slot::$variant(reader) => reader.expansions, )*
            }
        }
    };
}
ryo_relations::each_relation!(reader_slots);
