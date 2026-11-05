//! Borrowed numeric values, independent of query rows and source-language syntax.

/// A normalized signed decimal integer. Zero has no negative sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Decimal<'a> {
    /// Whether the nonzero magnitude is negative.
    pub negative: bool,
    /// Nonempty ASCII digits, without leading zeroes except for zero itself.
    pub digits: &'a str,
}

/// A numeric operand; whole-number cells retain their existing representation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Numeric<'a> {
    /// An arbitrary precision integer written as canonical decimal text.
    Decimal(Decimal<'a>),
    /// An existing signed whole-number cell or query literal.
    Whole(i64),
    /// A binary64 value, including nonfinite values and signed zero.
    Float(f64),
}
