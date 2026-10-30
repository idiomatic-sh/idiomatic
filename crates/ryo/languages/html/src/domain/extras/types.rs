//! The types of the `extras` concept: plain data, public fields and derives, with no constructor and no inherent impl. An extra is a kind
//! of the language's trees that its standard has no name for.

/// What a kind of a html syntax tree is that the standard lacks. One closed list, each name once.
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
    /// Bytes discarded by tokenizer recovery, retained for lossless reconstruction.
    Error,
}
