//! The pinned standard's closed extra vocabulary.
/// The elements of a JavaScript tree that are neither a node nor a token of the standard: white space and unread text.
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
pub(crate) enum Extra {
    /// `Whitespace` in the standard.
    Whitespace,
    /// `Error` in the standard.
    Error,
}
