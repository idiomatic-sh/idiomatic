//! What the adapter's writer asks of the tree it writes into: the old element table's lossless tree, and the walk's node table.

use super::types::Gap;
use crate::domain::syntax::types::Kind;
use ryo::languages::Offset;

/// What the gap tokenizer writes tokens through: the bytes between the events of the parser are white space or markup.
pub(super) trait Ops {
    /// Writes a token of `kind` over `from..to`.
    fn token(&mut self, kind: Kind, from: Offset, to: Offset);
    /// Writes a token as `token` does, with the text it carries.
    fn decoded(&mut self, kind: Kind, from: Offset, to: Offset, text: &str);
}

/// A tree the writer writes the parser's events into. Every byte before its end is in one token: the text no event spans is written
/// by the gap tokenizer before the next event.
pub(super) trait Tree {
    /// Opens a node of `kind` at `start`, after the gap before it, or where the last token ends when the event starts before it.
    fn enter(&mut self, kind: Kind, start: Offset, gap: &mut Gap<'_>);
    /// Closes the innermost node where the last token ends.
    fn leave(&mut self);
    /// Writes the gap from where the last token ends to `to`.
    fn fill_to(&mut self, to: Offset, gap: &mut Gap<'_>);
    /// Writes a token of `kind` from `start` to `end`, after the gap before it.
    fn token(&mut self, kind: Kind, start: Offset, end: Offset, gap: &mut Gap<'_>);
    /// Writes a token as `token` does, with the text the library read from it.
    fn decoded(&mut self, kind: Kind, start: Offset, end: Offset, text: &str, gap: &mut Gap<'_>);
    /// Writes the gap to `read`, then the bytes to `length` as text the parser could not read, and closes the document.
    fn finish(&mut self, read: Offset, length: Offset, gap: &mut Gap<'_>);
}
