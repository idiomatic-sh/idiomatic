//! What the writer of the adapter asks of the tree it writes into: the old element table's lossless tree, and the walk's node table.

use crate::domain::syntax::types::Kind;
use ryo::languages::Offset;

/// A tree the writer writes the parser's events into. Every byte before its end is in one token: the text no event spans is
/// written as text the parser could not read before the next event.
pub(in super::super) trait Sink {
    /// Where the last token written ends.
    fn end(&self) -> Offset;
    /// Opens a node of `kind` at `at`, after the gap before it, or where the last token ends when the event starts before it.
    fn enter(&mut self, kind: Kind, at: Offset);
    /// Closes the innermost node open where the last token ends.
    fn leave(&mut self);
    /// Writes a token of `kind` from `start` to `end`, after the gap before it.
    fn token(&mut self, kind: Kind, start: Offset, end: Offset);
    /// Writes the text no event spans up to `length`, and closes the document.
    fn finish(&mut self, length: Offset);
    /// Writes a token as `token` does, with the text its library decoded from it.
    fn decoded(&mut self, kind: Kind, start: Offset, end: Offset, text: &str);
}
