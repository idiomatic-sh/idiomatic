//! What the `syntax` port asks of the outside: go text in, the element table of its syntax tree
//! out.

use super::types::SourceText;
use ryo::languages::NodeTableBuilder;

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {

    /// Reads go text into the node table `builder` builds, numbered as the walk numbers it: the same elements as `read`, with no
    /// field; the walk gives them theirs once the table is written.
    fn walk(&self, text: SourceText<'_>, builder: &mut NodeTableBuilder<'_>);
}
