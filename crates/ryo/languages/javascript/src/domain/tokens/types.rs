//! The pinned standard's closed token vocabulary.
/// The kinds of token a JavaScript tree has, by the standard's names.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
pub(crate) enum Token {
    /// `IdentifierToken` in the standard.
    #[strum(serialize = "identifier-token")]
    Identifier,
    /// `PrivateIdentifierToken` in the standard.
    #[strum(serialize = "private-identifier-token")]
    PrivateIdentifier,
    /// `Keyword` in the standard.
    Keyword,
    /// `Numeric` in the standard.
    Numeric,
    /// `String` in the standard.
    String,
    /// `RegularExpression` in the standard.
    RegularExpression,
    /// `Template` in the standard.
    Template,
    /// `Punctuator` in the standard.
    Punctuator,
    /// `Line` in the standard.
    Line,
    /// `Block` in the standard.
    Block,
    /// `JSXIdentifierToken` in the JSX extension of the standard.
    #[strum(serialize = "jsx-identifier-token")]
    JSXIdentifier,
    /// `JSXTextToken` in the JSX extension of the standard.
    #[strum(serialize = "jsx-text-token")]
    JSXText,
}
