//! The types of the `parameter_kind` concept: the closed set of words, as an enum.

/// How a parameter takes its argument.
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
pub enum ParameterKind {
    /// The value a method is called on.
    Receiver,
    /// Passed by position only.
    Positional,
    /// Passed by position or by name.
    PositionalOrKeyword,
    /// Passed by name only.
    KeywordOnly,
    /// Collects the remaining positional arguments.
    VariadicPositional,
    /// Collects the remaining named arguments.
    VariadicKeyword,
    /// A named result the callable returns.
    Result,
}
