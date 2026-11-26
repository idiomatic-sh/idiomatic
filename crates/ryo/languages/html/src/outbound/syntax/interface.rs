//! What the `syntax` port asks of the outside: html text in, the numbered node table of its syntax tree out.

use super::types::SourceText;
use ryo::languages::{NodeTableBuilder, SourcePath};

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Reads html text into the numbered node table `builder` writes, each node with its role under its parent, and what the library
    /// said of the file. A file with errors still gives the nodes that could be read: reading never fails. What a URL-valued
    /// attribute points at is written on its node as the path of the run it names, resolved against `at`, the file's own path.
    fn walk(&self, text: SourceText<'_>, at: SourcePath<'_>, builder: &mut NodeTableBuilder<'_>);
}
