//! The names of the `numeric_literals` concept: the properties a literal node holds, and the kinds of number.

use ryo::languages::PropertyId;

/// The numeric kind of a literal node: [`INTEGER_NUMBER`] or [`FLOAT_NUMBER`].
pub(crate) const NUMERIC_KIND: PropertyId = PropertyId(1);
/// The value a literal node stands for: an integer's base-10 digits, a float's one binary64 spelling.
pub(crate) const NUMERIC_VALUE: PropertyId = PropertyId(2);
/// The type suffix written after a literal's digits (`u8`, `f32`); absent when none is written.
pub(crate) const NUMERIC_SUFFIX: PropertyId = PropertyId(3);

/// The numeric kind of an integer literal.
pub(crate) const INTEGER_NUMBER: &str = "integer";
/// The numeric kind of a float literal, a binary64 value.
pub(crate) const FLOAT_NUMBER: &str = "float";
