//! What the `syntax` port asks of the outside: typescript text and the dialect it is read in, the element table of its syntax tree
//! out.

use super::types::SourceText;
use crate::domain::syntax::types::Dialect;
use ryo::languages::TableBuilder;

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Reads typescript text into the table `builder` builds, a pre-order table of its nodes and tokens, each token's text interned. A file
    /// with errors still gives the elements that could be read, and what the library said of the rest: reading never fails.
    fn read(&self, text: SourceText<'_>, dialect: Dialect, builder: &mut TableBuilder<'_>);
}
