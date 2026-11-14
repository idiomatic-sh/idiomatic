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
