//! What the `syntax` port asks of the outside: Python text in, the element table of its syntax tree
//! out.

use super::types::{PythonSourceKind, SourceText};
use crate::settings::PythonParserSettings;
use ryo::languages::TableBuilder;

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Reads Python text into the table `builder` builds, a pre-order table of its nodes and tokens, each token's
    /// text interned. A file with errors still gives the elements that could be read, and what the library said of the rest: reading
    /// never fails.
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "the shared parser contract passes immutable language settings without consuming them"
    )]
    fn read(
        &self,
        text: SourceText<'_>,
        source_kind: PythonSourceKind,
        settings: &PythonParserSettings,
        builder: &mut TableBuilder<'_>,
    );
}
