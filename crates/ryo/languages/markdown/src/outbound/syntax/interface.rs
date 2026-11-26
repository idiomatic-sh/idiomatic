//! What the `syntax` port asks of the outside: Markdown text in, the element table of its syntax tree
//! out.

use super::types::SourceText;
use ryo::languages::SourcePath;


/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {

    /// Reads Markdown text into the node table `builder` builds, numbered as the walk numbers it; `at` is the file's own path.
    fn walk(&self, text: SourceText<'_>, at: SourcePath<'_>, decodings: &mut crate::domain::file_view::types::Decodings, builder: &mut ryo::languages::NodeTableBuilder<'_>);
}
