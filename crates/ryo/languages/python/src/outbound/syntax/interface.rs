//! What the `syntax` port asks of the outside: Python text in, the element table of its syntax tree
//! out.

use super::types::{PythonSourceKind, SourceText};
use crate::settings::PythonParserSettings;
use ryo::languages::TableBuilder;

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Own lazy parser-backed syntax rendering for the current admitted tree.
    fn begin_semantics(
        &self,
        _tree: &ryo::languages::Elements,
        _source_kind: PythonSourceKind,
        _settings: PythonParserSettings,
    ) -> Option<Box<dyn ryo::languages::SemanticSession>> {
        None
    }

    /// Parse only carriers already classified in a resolved type context, using the existing mapping writer.
    fn expand_annotations(
        &self,
        source: SourceText<'_>,
        carriers: &[super::types::DeferredAnnotation],
        builder: &mut TableBuilder<'_>,
    );

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
