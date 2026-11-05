//! Parser-derived numeric data without row storage or source-language evaluation.

/// One numeric literal's complete normalized payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NumericLiteral<'a> {
    /// `integer` or `float`.
    pub kind: &'static str,
    /// Canonical decimal integer or round-trip binary64 text.
    pub value: NumericValue<'a>,
}

/// A parser-owned integer slice or the bits of a parsed float.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NumericValue<'a> {
    Integer(&'a str),
    Float(u64),
}

/// No numeric occurrence, one complete occurrence, or a missing required parser property.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Classification<'a> {
    /// This element is not a supported integer or float occurrence.
    NotNumeric,
    /// Complete parser-owned numeric data.
    Known(NumericLiteral<'a>),
    /// A numeric occurrence cannot be read completely.
    MissingPayload,
}
