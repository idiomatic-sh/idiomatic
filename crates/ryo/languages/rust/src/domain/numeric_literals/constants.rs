//! The names of the `numeric_literals` concept: the properties a literal node holds.

use ryo::languages::PropertyId;

/// The numeric kind of a literal node: `integer` or `float`.
pub(crate) const NUMERIC_KIND: PropertyId = PropertyId(1);
/// The value a literal node stands for: an integer's base-10 digits, a float's one binary64 spelling.
pub(crate) const NUMERIC_VALUE: PropertyId = PropertyId(2);
/// The type suffix written after a literal's digits (`u8`, `f32`); absent when none is written.
pub(crate) const NUMERIC_SUFFIX: PropertyId = PropertyId(3);
