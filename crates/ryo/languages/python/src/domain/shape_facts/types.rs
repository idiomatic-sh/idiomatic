//! The types of the `shape_facts` concept: what the shapes of a file are built through.

use crate::domain::literal_facts::types::TextBuffer;
use ryo::languages::{FileWords, Position, RetainedKeyIndex, ShapeWord};

/// The shapes of a file being built: the classes found so far, the file's words and a text buffer.
#[derive(Debug)]
pub(crate) struct ShapeBuild<'b> {
    /// The classes by node id, and the key in hand.
    pub index: &'b mut RetainedKeyIndex<Position, ShapeWord>,
    /// The words of the file: a name, a literal's value and an operator are each one.
    pub words: &'b mut FileWords,
    /// A text built for one literal.
    pub text: &'b mut TextBuffer,
}

/// Whether literal text of an interpolated string is waiting to be written: adjacent literal pieces are one piece of structure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Pending {
    /// Nothing waits.
    #[default]
    Nothing,
    /// The text in the buffer waits.
    Text,
}

/// How a node's shape is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShapeReading {
    /// By its value: a name's spelling, a string's decoded text, a scalar's payload.
    ByValue,
    /// By its children and the operators between them.
    ByChildren,
}
