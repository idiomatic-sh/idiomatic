//! The types of the `literals` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::borrow::Cow;

/// Text of a file as it is written: a string literal, or the part of one still to be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Written<'a>(pub &'a str);

/// The text a string literal stands for: borrowed from the file when nothing in it is an escape, made when something is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Unquoted<'a>(pub Cow<'a, str>);

/// What opens and closes a string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Delimiter(pub char);

/// How many digits an escape has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Digits(pub usize);

/// The base the digits of an escape are written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Radix(pub u32);

/// The number the digits of an escape write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Code(pub u32);

/// A character an escape stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Glyph(pub char);

/// A byte an escape stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Octet(pub u8);

/// The digits of an escape read, and the text that follows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Numbered<'a> {
    /// What they write.
    pub code: Code,
    /// What follows.
    pub after: Written<'a>,
}

/// What one escape stands for, and the text that follows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Escaped<'a> {
    /// A character, which any of the escapes makes.
    Character {
        /// The character.
        glyph: Glyph,
        /// What follows.
        after: Written<'a>,
    },
    /// A byte, which `\x` and the octal escapes make.
    Byte {
        /// The byte.
        octet: Octet,
        /// What follows.
        after: Written<'a>,
    },
}
