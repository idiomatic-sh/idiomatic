//! What the `syntax` port asks of the outside: Markdown text in, the element table of its syntax tree
//! out.

use super::types::SourceText;
use ryo::languages::SourcePath;
use ryo::languages::TableBuilder;

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Reads Markdown text into the table `builder` builds, a pre-order table of its nodes and tokens, each token's
    /// text interned, every byte of the file in one token. Every text is a Markdown document; bytes that are not text are kept, and
    /// what was said of them: reading never fails. The destination of a link or an image is written on its first marker as the path
    /// of the run it names, resolved against `at`, the file's own path.
    fn read(&self, text: SourceText<'_>, at: SourcePath<'_>, builder: &mut TableBuilder<'_>);
}
