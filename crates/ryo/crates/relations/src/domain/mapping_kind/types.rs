//! The types of the `mapping_kind` concept: the closed set of words, as an enum.

/// How a mapping of a data document is written.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
    strum::EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u8)]
pub enum MappingKind {
    /// Opened by its own header.
    Standard,
    /// Never written, only implied by a longer header.
    Implicit,
    /// Opened by a dotted key.
    Dotted,
    /// Written inline as a value.
    Inline,
    /// One member of a list of mappings.
    SequenceMember,
}
