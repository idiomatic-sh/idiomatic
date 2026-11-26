//! What the `syntax` port asks of the outside: TOML text in, the element table of its syntax tree
//! out.

use super::types::SourceText;


/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {

    /// Reads TOML text into the node table `builder` builds, numbered as the walk numbers it. `values` says whether a relation reads
    /// the decoded keys and scalars, which the tree then carries.
    fn walk(&self, text: SourceText<'_>, values: bool, decodings: &mut crate::domain::file_view::types::Decodings, builder: &mut ryo::languages::NodeTableBuilder<'_>);
}
