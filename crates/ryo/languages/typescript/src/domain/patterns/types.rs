//! The types of the `patterns` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The role of a pattern element under its parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatternRole {
    /// What the parent holds: its alternatives, terms, contents or quantified term.
    Body,
    /// The low end of a range.
    Min,
    /// The high end of a range.
    Max,
}

/// The part of what a carrier gives the library that the library refused: it reads the flags first and the pattern only when it
/// takes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RefusedPart {
    /// The pattern.
    Pattern,
    /// The flags.
    Flags,
}

/// What the library refused of a carrier, as the carrier's properties say: the part and the library's message.
#[derive(Clone, Copy, Debug)]
pub(super) struct Refusal<'t> {
    /// The part refused.
    pub part: RefusedPart,
    /// The library's message.
    pub message: ryo::languages::NameText<'t>,
}
