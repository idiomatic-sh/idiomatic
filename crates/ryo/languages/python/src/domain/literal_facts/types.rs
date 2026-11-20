//! The types of the `literal_facts` concept: what a number is, what a string holds, and the literal pieces of an f-string.

use ryo::languages::{Flag, NameText, Position};

/// A text a filler builds for one row and reuses for the next.
#[derive(Clone, Debug, Default)]
pub(crate) struct TextBuffer(pub String);

/// A numeric literal: its kind and whether the digits written survive the value it stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NumberFacts {
    /// `integer` or `float`.
    pub kind: NameText<'static>,
    /// Whether the written digits are the value's: always for an integer.
    pub exact: Flag,
}

/// What a string, an f-string or a t-string holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StringValue<'b> {
    /// Its parts joined as the parser decoded them.
    Known(NameText<'b>),
    /// It interpolates: its value is made when it runs.
    Interpolated,
}

/// A literal piece of an f-string: a plain string part, or the literal text between its interpolations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LiteralPiece<'n> {
    /// The piece's node.
    pub id: Position,
    /// Its text as the parser decoded it.
    pub value: NameText<'n>,
}
