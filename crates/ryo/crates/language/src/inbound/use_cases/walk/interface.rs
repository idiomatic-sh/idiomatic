//! What `walk` promises: the one contract a language implements for the engine. It lives beside `parse` until the swap landing,
//! which deletes `parse`.

use super::errors::WalkError;
use super::types::{FileVisit, RelationNeeds, WalkRequest};
use crate::domain::fillers::types::Fillers;
use crate::domain::node_table::types::NodeTableBuilder;
use ryo_relations::domain::relations::types::Count;

/// What a language offers the engine for its files: its walk, over its fillers (which hold its per-file view, `File<'f>`).
///
/// # Examples
///
/// ```
/// use ryo_language::inbound::use_cases::walk::interface::WalkUseCase;
/// use ryo_language::domain::fillers::fixtures::Walking;
///
/// assert_eq!(<Walking as WalkUseCase>::KIND_COUNT.0, 3);
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
}
