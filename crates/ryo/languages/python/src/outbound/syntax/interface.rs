//! What the `syntax` port asks of the outside: Python text in, the element table of its syntax tree
//! out.

use super::types::{PythonSourceKind, SourceText};
use ryo::languages::TableBuilder;

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Own lazy parser-backed syntax rendering for the current admitted tree, over the parsed tree `read` kept in `retained`.
    fn begin_semantics(
        &self,
        _tree: &ryo::languages::Elements,
        _source_kind: PythonSourceKind,
        _retained: ryo::languages::SemanticSlot,
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
    /// text interned, and keeps the parsed tree in the table for a semantic session of the file. A file with errors still gives the elements that could be read, and what the library said of the rest: reading
    /// never fails.
    fn read(
        &self,
        text: SourceText<'_>,
        source_kind: PythonSourceKind,
        builder: &mut TableBuilder<'_>,
    );
}
