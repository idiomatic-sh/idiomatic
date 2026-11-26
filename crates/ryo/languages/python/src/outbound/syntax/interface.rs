//! What the `syntax` port asks of the outside: Python text in, the element table of its syntax tree
//! out.

use super::types::{PythonSourceKind, ReportedSyntax, SourceText};
use crate::domain::node::types::Walked;
use ryo::languages::{NodeTableBuilder, Position};

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
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
