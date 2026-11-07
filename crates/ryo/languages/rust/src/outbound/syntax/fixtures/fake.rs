//! The fake of the `syntax` port: what the language's tests stand in for it with. It replays a tree written by hand (`script.rs`) and
//! reads nothing of the text it is given, so the text of a request must be the text the tree spells. It reads no macro again, and the value of
//! a plain string token (no escape) is the text between its quotes.

use super::super::interface::SyntaxPort;
use super::super::types::SourceText;
use super::script::{Piece, replay};
use ryo::languages;

/// What the language's tests stand in for `syntax` with: it makes the tree it was given.
pub(crate) struct SyntaxFake(pub Piece);

impl SyntaxPort for SyntaxFake {
    fn read(&self, _text: SourceText<'_>, builder: &mut languages::TableBuilder<'_>) {
        replay(&self.0, builder);
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
