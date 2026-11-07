//! What the `syntax` port asks of the outside: CSS text in, the element table of its syntax tree out.

use super::types::SourceText;
use ryo::languages::SourcePath;
use ryo::languages::TableBuilder;

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Reads CSS text into the table `builder` builds, a pre-order table of its nodes and tokens, each token's text interned. A file
    /// with errors still gives every element, and what the library said of the rest: reading never fails. What an `@import` imports
    /// and what a `url()` points at is written on its first token as the path of the run it names, resolved against `at`, the file's
    /// own path.
    fn read(&self, text: SourceText<'_>, at: SourcePath<'_>, builder: &mut TableBuilder<'_>);
}
