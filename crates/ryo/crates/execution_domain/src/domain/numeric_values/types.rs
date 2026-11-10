//! Borrowed numeric values, independent of query rows and source-language syntax.

use ryo_values::domain::plan_parts::types::WholeNumber;

/// Whether a nonzero magnitude is negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Negative(pub bool);

/// Nonempty ASCII digits, without leading zeroes except for zero itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DecimalDigits<'a>(pub &'a str);

/// Text offered to be read as a number, not yet known to be canonical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NumberText<'a>(pub &'a str);

/// A binary64 value, including nonfinite values and signed zero.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct FloatValue(pub f64);

/// A nonnegative integer magnitude that fits 128 bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Magnitude(pub u128);

/// A left shift by a count of binary digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shift(pub u32);

/// A signed integer that fits 128 bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SmallInteger(pub i128);

/// Whether text has a decimal binary64 spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FloatForm {
    /// A digit run with a dot or an exponent marker, in full.
    Spelled,
    /// Anything else.
    Malformed,
}

/// Whether one integer is exactly one more than another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Succession {
    /// The right integer is the left plus one.
    Successor,
    /// It is not, or one side is not an integer.
    NotSuccessor,
}

/// A normalized signed decimal integer. Zero has no negative sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Decimal<'a> {
    /// Whether the nonzero magnitude is negative.
    pub negative: Negative,
    /// The magnitude's digits.
    pub digits: DecimalDigits<'a>,
}

/// A numeric operand; whole-number cells retain their existing representation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Numeric<'a> {
    /// An arbitrary precision integer written as canonical decimal text.
    Decimal(Decimal<'a>),
    /// An existing signed whole-number cell or query literal.
    Whole(WholeNumber),
    /// A binary64 value, including nonfinite values and signed zero.
    Float(FloatValue),
}
