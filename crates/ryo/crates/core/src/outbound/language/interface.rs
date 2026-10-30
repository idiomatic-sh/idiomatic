//! What the `language` port asks of the outside: the relations ryo owns, and files and manifests
//! read into typed rows. Which languages are linked is the adapter's business, never the caller's.

use crate::outbound::language::types::{Answers, LinkedNames, RelationList, UnknownReasons};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::relations::types::Relation;
use ryo_language::domain::relations::types::RowWriter;
use ryo_language::domain::symbols::types::Symbols;

use super::errors::LanguageError;
use super::types::{FileReading, FileToRead, ManifestReading, ProjectAnswer, ProjectInput};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::descriptions::types::KindVocabularies;

/// What every `language` adapter implements.
pub trait LanguagePort: Send + Sync {
    /// Lists the relations ryo owns, whatever languages are linked.
    ///
    /// # Errors
    ///
    /// `Catalog` when the catalog cannot be read.
    fn relations(&self) -> Result<RelationList, LanguageError>;

    /// The names of the languages linked, in the order they were linked.
    fn linked(&self) -> LinkedNames;

    /// The kinds each linked language has, in the order they were linked: what a query may name when it compares the kind of a node.
    fn kind_vocabularies(&self) -> KindVocabularies;

    /// The reasons the linked languages declare for a value they cannot resolve statically, every language's together, each once.
    fn unknown_reasons(&self) -> UnknownReasons;

    /// What every linked language answers for `relation`, in the order they were linked.
    fn supports(&self, relation: Relation) -> Answers;

    /// Whether a linked language reads the file at `path` as a manifest: the names are the languages' own.
    fn reads_manifest(&self, path: &FilePath) -> ManifestReading;

    /// Reads one file with the language that reads it: the tree goes into `table`, the worker's own, and the rows into `out`. A file no language
    /// reads is `Unsupported`; a file a language fails on is `Failed`, not an error: the run goes
    /// on and lists it as unread.
    fn parse(
        &self,
        file: FileToRead<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        out: &mut dyn RowWriter,
    ) -> FileReading;

    /// Reads the manifests with every linked language, writing into `out`.
    ///
    /// # Errors
    ///
    /// `Manifest` when a language's tools cannot run at all; a manifest that does not parse is refused in the answer, and the others are read.
    ///
    /// The answer is the manifests refused, the manifests the rows are about that the run does not list, and the files the languages
    /// read beyond the manifests. Of it the store keeps only the digests of those files: a run reads the project again when one of
    /// them, a manifest or what a file notes for the project changed.
    fn project(
        &self,
        input: &ProjectInput<'_>,
        symbols: &mut Symbols,
        out: &mut dyn RowWriter,
    ) -> Result<ProjectAnswer, LanguageError>;
}
