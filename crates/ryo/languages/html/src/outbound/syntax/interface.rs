//! What the `syntax` port asks of the outside: html text in, the element table of its syntax tree
//! out.

use super::types::SourceText;
use ryo::languages::{SourcePath, TableBuilder};

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Reads html text into the table `builder` builds, a pre-order table of its nodes and tokens, each token's text interned. A file
    /// with errors still gives the elements that could be read, and what the library said of the rest: reading never fails. What a
    /// URL-valued attribute points at is written on its node as the path of the run it names, resolved against `at`, the file's own
    /// path.
    fn read(&self, text: SourceText<'_>, at: SourcePath<'_>, builder: &mut TableBuilder<'_>);
}
