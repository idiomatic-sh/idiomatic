//! The types of the `value_kind` concept: the closed set of words, as an enum.

/// What a value of a data document is.
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
pub enum ValueKind {
    /// Text.
    String,
    /// A whole number.
    Integer,
    /// A number with a fraction.
    Float,
    /// True or false.
    Boolean,
    /// A date, a time, or both.
    Datetime,
    /// An ordered list of values.
    Sequence,
    /// A mapping written inline as a value.
    InlineMapping,
    /// A mapping the key opens.
    Mapping,
    /// A list of mappings the key opens.
    SequenceOfMappings,
    /// The value that says there is none: JSON's `null`. TOML has no such value.
    Null,
}
