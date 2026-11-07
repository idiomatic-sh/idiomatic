//! The types of the `tokens` concept: plain data, public fields and derives, with no constructor and no inherent impl. A token is named
//! as the standard names it, in kebab case.

/// What a token of a typescript syntax tree is. One closed list, each name once.
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
#[expect(
    clippy::enum_variant_names,
    reason = "The standard token names collide with AST node names; the Token suffix keeps the two vocabularies distinct"
)]
pub(crate) enum Token {
    /// The standard’s `Boolean` kind.
    Boolean,
    /// The standard’s `IdentifierToken` kind.
    IdentifierToken,
    /// The standard’s `JSXIdentifierToken` kind.
    JSXIdentifierToken,
    /// The standard’s `PrivateIdentifierToken` kind.
    PrivateIdentifierToken,
    /// The standard’s `JSXTextToken` kind.
    JSXTextToken,
    /// The standard’s `Keyword` kind.
    Keyword,
    /// The standard’s `Null` kind.
    Null,
    /// The standard’s `Numeric` kind.
    Numeric,
    /// The standard’s `Punctuator` kind.
    Punctuator,
    /// The standard’s `RegularExpression` kind.
    RegularExpression,
    /// The standard’s `String` kind.
    String,
    /// The standard’s `Template` kind.
    Template,
    /// The standard’s `Block` kind.
    Block,
    /// The standard’s `Line` kind.
    Line,
}
