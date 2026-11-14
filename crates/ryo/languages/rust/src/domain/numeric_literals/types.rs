//! The types of the `numeric_literals` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::{Flag, FloatBits, NameText};

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
        /// Whether the value printed to as many significant digits as the literal writes gives them back: always for an
        /// integer.
        exact: Flag,
    },
}

/// The value of an integer literal: every Rust integer literal a compiler accepts fits 128 bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IntegerValue(pub u128);

/// What a numeric literal's digits stand for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumberValue {
    /// An integer.
    Integer(IntegerValue),
    /// A float, as the binary64 value of its digits (an `f32` literal's value after rounding to `f32`).
    Float(FloatBits),
}

/// A numeric literal as Rust reads it: its value, and the type suffix written after its digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DecodedNumber<'a> {
    /// The value its digits stand for.
    pub value: NumberValue,
    /// The suffix written after the digits; none when no type is written.
    pub suffix: Option<NameText<'a>>,
}
