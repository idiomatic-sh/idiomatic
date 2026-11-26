//! What the `syntax` port asks of the outside: typescript text and the dialect it is read in, the element table of its syntax tree
//! out.

use super::types::{SourceText, Wants, Walked};
use crate::domain::syntax::types::Dialect;
use ryo::languages::{NodeTableBuilder};

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {

    /// With `flow`, the control-flow facts are kept for the fillers. Reads typescript text into the numbered node table `builder` writes, each node with its role under its parent. Reading never
    /// fails: a file with errors still gives the nodes that could be read, and what the library said of the rest.
    fn walk(&self, text: SourceText<'_>, dialect: Dialect, builder: &mut NodeTableBuilder<'_>, wants: Wants) -> Walked;
}
