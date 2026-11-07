//! What the `syntax` port asks of the outside: go text in, the element table of its syntax tree
//! out.

use super::types::SourceText;
use ryo::languages::TableBuilder;

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Reads go text into the table `builder` builds, a pre-order table of its nodes and tokens, each token's text interned. A file
    /// with errors still gives the elements that could be read, and what the library said of the rest: reading never fails.
    fn read(&self, text: SourceText<'_>, builder: &mut TableBuilder<'_>);
}
