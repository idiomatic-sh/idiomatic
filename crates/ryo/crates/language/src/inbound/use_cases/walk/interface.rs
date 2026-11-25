//! What `walk` promises: the one contract a language implements for the engine. It lives beside `parse` until the swap landing,
//! which deletes `parse`.

use super::errors::WalkError;
use super::types::{FileVisit, RelationNeeds, WalkRequest};
use crate::domain::fillers::types::Fillers;
use crate::domain::node_table::types::NodeTableBuilder;
use crate::domain::files::types::SourcePath;
use crate::inbound::use_cases::parse::named::{FileRelation, SourceWords};
use crate::inbound::use_cases::parse::types::{ParseConfigurationIdentity, Recognition, SkippedDirectory};
use ryo_relations::domain::catalog::types::OptionalCapability;
use ryo_relations::domain::relations::types::{Count, Relation};
use ryo_relations::domain::terms::types::UnknownName;

/// What a language offers the engine for its files: its walk, over its fillers (which hold its per-file view, `File<'f>`).
///
/// # Examples
///
/// ```
/// use ryo_language::inbound::use_cases::walk::interface::WalkUseCase;
/// use ryo_language::domain::fillers::fixtures::Walking;
///
/// assert_eq!(<Walking as WalkUseCase>::KIND_COUNT.0, 4);
/// ```
pub trait WalkUseCase: Fillers {
    /// Per-worker buffers the language refills for each file (the parse, the model's arenas).
    type Scratch: Default + Send + 'static;

    /// How many kinds of node and token the language's trees have: sizes `KindsPresent`.
    const KIND_COUNT: Count;

    /// Parses the file, writes its node table through `builder`, then lends the file to `visit` while the parse lives: the language
    /// finishes the table (`node_table::service::finished`) and hands `visit` the table, the bytes and its fillers over its view
    /// (`fillers::service::file_source`). `needs` names the relations some loaded program reads, so a language skips the side
    /// indexes nobody can read.
    ///
    /// # Errors
    ///
    /// `Unsupported` when the language does not read the file; `Library` when the parser fails.
    fn walk(
        &self,
        request: WalkRequest<'_>,
        needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        scratch: &mut Self::Scratch,
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError>;

    /// Whether the language reads the file at `path`: `explicit` says the file was named, not found by a walk of the repository.
    fn recognizes(&self, path: SourcePath<'_>, explicit: bool) -> Recognition;

    /// The directories a walk of the repository does not enter for this language.
    fn skips(&self) -> &'static [SkippedDirectory];

    /// What the language's configuration is, so a result made under another is not reused.
    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_>;

    /// The optional capabilities of the catalog the language supplies.
    fn capabilities(&self) -> &'static [OptionalCapability];

    /// The reasons the language declares for a value it cannot resolve statically, by their place in its list.
    fn unknown_reasons(&self) -> &'static [UnknownName];

    /// How a text is a name in this language, when a cell of a column may be proven missing from the file by its words.
    fn source_words(&self) -> Option<SourceWords> {
        None
    }

    /// Whether the rows of `relation` cover the nodes the parser embeds (a string read as code), not only the file's own.
    fn includes_expansions(&self, _relation: Relation) -> bool {
        false
    }

    /// The file-scope relations the language declares beside the catalog's (see `Fillers::named`).
    fn named_declarations(&self) -> &'static [FileRelation] {
        &[]
    }

    /// Whether any of those relations consults the repository's observations.
    fn uses_project(&self) -> bool {
        false
    }
}
