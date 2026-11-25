//! What the `syntax` port asks of the outside: typescript text and the dialect it is read in, the element table of its syntax tree
//! out.

use super::types::SourceText;
use crate::domain::syntax::types::Dialect;
use ryo::languages::{NodeTableBuilder, TableBuilder};

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Reads typescript text into the table `builder` builds, a pre-order table of its nodes and tokens, each token's text interned. A file
    /// with errors still gives the elements that could be read, and what the library said of the rest: reading never fails.
    fn read(&self, text: SourceText<'_>, dialect: Dialect, builder: &mut TableBuilder<'_>);

    /// Reads typescript text into the numbered node table `builder` writes, each node with its role under its parent. Reading never
    /// fails: a file with errors still gives the nodes that could be read, and what the library said of the rest.
    #[cfg_attr(not(test), allow(dead_code, reason = "the walk is linked when step 0 of the ports lands"))]
    fn walk(&self, text: SourceText<'_>, dialect: Dialect, builder: &mut NodeTableBuilder<'_>);
}
