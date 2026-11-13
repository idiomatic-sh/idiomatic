//! The names of the parse contract.
use ryo::languages;

/// The optional capabilities the typescript language answers.
pub(crate) const CAPABILITIES: &[languages::OptionalCapability] = &[
    languages::OptionalCapability::NumericLiteral,
    languages::OptionalCapability::ExpressionShape,
];
