//! What the `parse` use case promises of a language besides its walk: which files it reads, which directories it skips, the kinds its
//! trees have, the reasons it declares for a value it cannot resolve, and the optional capabilities it promises. Its trees and rows
//! come from its walk (`walk`), whose fillers write every relation; the old reader that wrote them beside it is gone.

use super::types::{KindNames, ParseConfigurationIdentity, ParseRequest, SkippedDirectory};
use crate::inbound::use_cases::parse::types::Recognition;
use ryo_relations::domain::relations::types::Count;
use ryo_relations::domain::terms::types::UnknownName;

/// What a language offers for its files.
pub trait ParseUseCase: Send + Sync {
    /// Necessary spelling guarantees for this implementation's syntax and fact columns.
    fn source_words(&self) -> Option<super::named::SourceWords> { None }

    /// Provider-owned file relations beyond the shared catalog.
    fn file_relations(&self) -> Option<&dyn super::named::FileRelations> { None }


    /// The optional capabilities of the language this provider promises to answer, the optional relations of its trees among them (the
    /// relations the catalog declares `optional` that its walk fills), and of the project's relations, including proven language absence.
    fn capabilities(&self) -> &'static [ryo_relations::domain::catalog::types::OptionalCapability];

    /// Canonical immutable parser settings that can change the tree or diagnostics.
    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_>;







    /// Whether the language reads the file the request names, by its path.
    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition;

    /// Whether this provider accepts a file explicitly assigned to it by the host.
    /// A language that does not accept explicit selection answers `Declines`, which never bypasses path recognition.
    fn recognizes_explicit(&self, request: &ParseRequest<'_>) -> Recognition;

    /// The names of the directories the language never reads, at any depth: the dependency, environment and build directories of its
    /// ecosystem (`node_modules`, `target`), which both ryo's walk of a repository and Tastemaker's do not enter. A language whose
    /// ecosystem has none answers an empty list, and says in its constants why.
    fn skips(&self) -> &'static [SkippedDirectory];

    /// How many kinds of node and token the language's trees have, in its own numbering. It is at most the
    /// number of kinds a walk can tell apart.
    fn kind_count(&self) -> Count;

    /// The names of the kinds the language's trees have, each once: the words a query can write for a kind of node or token. A name
    /// the language's trees have no kind of is a word no query can match, which the engine refuses when a query writes it.
    fn kind_names(&self) -> KindNames;

    /// Field codes whose meaning stays fixed across files. Providers with file-local
    /// dictionaries leave this empty. File vocabulary checks can still use their names.
    fn stable_fields(&self) -> crate::domain::fillers::types::FieldNames {
        Default::default()
    }

    /// The reasons the language declares for a value of a column it cannot resolve statically (a wildcard import, a call of a variable):
    /// a closed list, each named once and shown by a fixture row. A reader that meets such a construct writes its row with the
    /// unknown value of the reason, and never omits the row or marks the whole file partial.
    fn unknown_reasons(&self) -> &'static [UnknownName];



}
