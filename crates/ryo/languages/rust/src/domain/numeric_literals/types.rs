//! The types of the `numeric_literals` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::NameText;

/// What a literal node of a file's tree holds of a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiteralNumber<'a> {
    /// The literal is no number: a string, a character, a byte, `true` or `false`.
    NotNumeric,
    /// The literal is a number a compiler refuses, which the adapter gave no value.
    Unread,
    /// The number's kind and value, as the adapter stored them.
    Known {
        /// `integer` or `float`.
        kind: NameText<'a>,
        /// The integer's base-10 digits, or the float's one binary64 spelling.
        value: NameText<'a>,
    },
}
