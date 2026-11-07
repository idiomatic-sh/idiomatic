//! Borrowed text and widths of one fit check; no engine authority.

/// The rendered candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateText<'a>(pub &'a str);

/// The source text on the candidate's first line before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinePrefix<'a>(pub &'a str);

/// The widest permitted line, in columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineWidth(pub usize);

/// Columns between tab stops; never zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabStop(pub usize);

/// A column within a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LineColumn(pub usize);

/// Whether a candidate stays within the line width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    /// Every line is within the width.
    Within,
    /// Some line is wider.
    Over,
}
