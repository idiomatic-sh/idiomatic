//! Parser-derived numeric data without row storage or source-language evaluation.
use ryo::languages::{FloatBits, NameText};

/// One numeric literal's complete normalized payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NumericLiteral<'a> {
    /// `integer` or `float`.
    pub kind: NameText<'static>,
    /// Canonical decimal integer or round-trip binary64 text.
    pub value: NumericValue<'a>,
}

/// A parser-owned integer slice or the bits of a parsed float.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NumericValue<'a> {
    Integer(NameText<'a>),
    Float(FloatBits),
}

/// No numeric occurrence, one complete occurrence, or a missing required parser property.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NumericClassification<'a> {
    /// This element is not a supported integer or float occurrence.
    NotNumeric,
    /// Complete parser-owned numeric data.
    Known(NumericLiteral<'a>),
    /// A numeric occurrence cannot be read completely.
    MissingPayload,
}
