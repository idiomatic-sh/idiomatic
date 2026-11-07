//! The types of the `extras` concept: plain data, public fields and derives, with no constructor and no inherent impl. An extra is a kind of the language's trees that rust-analyzer's parser has no name for.

/// What a kind of a Rust syntax tree is that the standard lacks. One closed list, each name once. `Other` is what the parser's three markers (`TOMBSTONE`, `EOF`, `__LAST`), which no tree it builds holds, map to.
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
    /// The kind the parser's markers map to, which no built tree holds.
    Other,
}
