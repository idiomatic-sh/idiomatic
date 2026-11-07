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

/// What a fit check reads from its operation: the text columns and the literals between them, the span the
/// candidate replaces, the widest line and the tab stop.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RenderedShape<'a> {
    /// The columns whose texts the candidate interleaves.
    pub texts: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
    /// The literals before, between and after the texts: one more than the texts.
    pub literals: &'a [ryo_values::domain::plan_parts::types::LiteralText],
    /// The column of the span the candidate replaces.
    pub span: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The widest permitted line.
    pub max_line: ryo_values::domain::query_operators::types::MaxLine,
    /// The columns between tab stops.
    pub tab_width: ryo_values::domain::query_operators::types::TabWidth,
}
