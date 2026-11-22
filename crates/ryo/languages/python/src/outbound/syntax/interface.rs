//! What the `syntax` port asks of the outside: Python text in, the element table of its syntax tree
//! out.

use super::types::{PythonSourceKind, ReportedSyntax, SourceText};
use crate::domain::node::types::Walked;
use ryo::languages::{NodeTableBuilder, Position, TableBuilder};

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Own lazy parser-backed syntax rendering for the current admitted tree, over the parsed tree `read` kept in `retained`.
    fn begin_semantics(
        &self,
        _tree: &ryo::languages::Elements,
        _source_kind: PythonSourceKind,
        _retained: ryo::languages::SemanticSlot,
    ) -> ryo::languages::SemanticSlot {
        ryo::languages::SemanticSlot::Empty
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

    /// Walks Python text into the node table `builder` builds: every node as the library's walk opens it, with the field it fills,
    /// and its tokens between the nodes; reports what the library and the language said of the file, as much as `reported` asks.
    /// It then asks `read` which of the strings it met the language reads as type expressions, parses each and writes its
    /// expression as an embedded segment of the table, and asks again for the strings of those expressions, until none is one;
    /// then it hands `read` the walked tree, the typed node of each place among it, while the parses live. A file with errors
    /// still gives the nodes that could be read: walking never fails. `names` are the buffers the walk interns into (cleared
    /// by the caller, kept from file to file); the tree it lends holds them.
    fn walk(
        &self,
        text: SourceText<'_>,
        source_kind: PythonSourceKind,
        reported: ReportedSyntax,
        builder: &mut NodeTableBuilder<'_>,
        names: crate::domain::model::names::types::WalkNames,
        read: &mut dyn FnMut(Walked<'_, '_, '_>) -> Vec<Position>,
    );
}
