//! The words of the `literals` concept: how the language writes a string.

use super::types::{Delimiter, Digits, Radix};

/// What opens and closes an interpreted string.
pub(crate) const QUOTE: Delimiter = Delimiter('"');

/// What opens and closes a raw string.
pub(crate) const BACK_QUOTE: Delimiter = Delimiter('`');

/// What opens an escape in an interpreted string.
pub(crate) const ESCAPE: char = '\\';

/// The carriage return a raw string does not keep.
pub(crate) const RETURN: char = '\r';

/// What a raw string's carriage returns are replaced with.
pub(crate) const NOTHING: &str = "";

/// The digits of an octal escape.
pub(crate) const OCTAL_DIGITS: Digits = Digits(3);

/// The digits of a `\x` escape.
pub(crate) const HEX_DIGITS: Digits = Digits(2);

/// The digits of a `\u` escape.
pub(crate) const SHORT_UNICODE_DIGITS: Digits = Digits(4);

/// The digits of a `\U` escape.
pub(crate) const LONG_UNICODE_DIGITS: Digits = Digits(8);

/// The base of octal digits.
pub(crate) const OCTAL: Radix = Radix(8);

/// The base of hexadecimal digits.
pub(crate) const HEXADECIMAL: Radix = Radix(16);
