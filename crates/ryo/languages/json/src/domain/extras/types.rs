//! The types of the `extras` concept: plain data, public fields and derives, with no constructor and no inherent impl. An extra is a kind
//! of the language's trees that its standard has no name for.

/// What a kind of a json syntax tree is that the standard lacks. One closed list, each name once.
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
    /// Text the parser could not read as any token: a document that is not json has it, and the standard describes only documents that are.
    Error,
    /// A comment that runs to the end of the line, `// ...`: JSON with comments reads it, and the standard has no comment.
    LineComment,
    /// A comment between `/*` and `*/`: JSON with comments reads it, and the standard has no comment.
    BlockComment,
}
