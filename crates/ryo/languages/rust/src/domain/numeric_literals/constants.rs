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

/// The suffixes that make a numeric literal an integer of a type: the Rust Reference, Integer literals.
pub(crate) const INTEGER_SUFFIXES: [&str; 12] = [
    "u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64", "u128", "i128", "usize", "isize",
];

/// The suffix of a 32-bit float, whose value is rounded to `f32` before it is written as binary64.
pub(crate) const F32_SUFFIX: &str = "f32";

/// The suffix of a 64-bit float.
pub(crate) const F64_SUFFIX: &str = "f64";

/// The prefixes of the integer literals in another base than ten, each with its base.
pub(crate) const BASES: [(&str, u32); 3] = [("0x", 16), ("0o", 8), ("0b", 2)];
