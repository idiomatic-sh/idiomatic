//! What the writer of the adapter asks of the tree it writes into: the old element table's builder, and the walk's node table.

use crate::domain::syntax::types::Kind;
use ryo::languages::Offset;

/// A tree the writer writes the library's callbacks and tokens into.
pub(in super::super) trait Out {
    /// Opens a node of `kind` at `at`.
    fn enter(&mut self, kind: Kind, at: Offset);
    /// Closes the innermost node at `at`.
    fn leave(&mut self, at: Offset);
    /// Writes a token of `kind` over `from..to`, with the text the library decoded from it when it is given.
    fn token(&mut self, kind: Kind, from: Offset, to: Offset, decoded: Option<&str>);
    /// Records what is said of the file at `from..to`.
    fn diagnose(&mut self, message: &str, from: Offset, to: Offset);
}
