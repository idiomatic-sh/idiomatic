//! What the `embedded` port asks of the outside: the blocks of javascript a file of another format carries.

use super::types::{Blocks, Markup};
use ryo::languages::{Recognition, SourcePath};

/// What every `embedded` adapter implements.
pub(crate) trait EmbeddedPort: Send + Sync {
    /// Whether the adapter reads the format of the file at `path`.
    fn claims(&self, path: SourcePath<'_>) -> Recognition;

    /// The blocks of `text`, in file order, each with its start in the file. A file with no block gives none.
    fn blocks<'a>(&self, text: Markup<'a>) -> Blocks<'a>;
}
