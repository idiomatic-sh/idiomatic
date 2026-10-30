//! The types of the `tokens` concept: plain data, public fields and derives, with no constructor and no inherent impl. A token is named
//! as the standard names it, in kebab case.

/// What a token of a html syntax tree is. One closed list, each name once.
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
    /// A document type token.
    #[strum(serialize = "doctype-token")]
    Doctype,
    /// An opening tag token.
    StartTag,
    /// A closing tag token.
    EndTag,
    /// A comment token.
    #[strum(serialize = "comment-token")]
    Comment,
    /// Consecutive character tokens sharing a contiguous source range.
    Characters,
}
