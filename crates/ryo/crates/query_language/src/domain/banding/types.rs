//! The types of the `banding` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// One `l.a < r.b` of a join's `where`, as written: a column of each side and how the right one compares with the left one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WrittenEdge {
    /// The left side's column.
    pub left: ryo_values::domain::query_syntax::types::ColumnReference,
    /// How the right value compares with the left one.
    pub test: ryo_values::domain::band_joins::types::BandTest,
    /// The right side's column.
    pub right: ryo_values::domain::query_syntax::types::ColumnReference,
}

/// A join's `where` split: its band, the edges a band join answers, and the conditions left for the filter after it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BandSplit {
    /// The band: each `l.a < r.b` between the sides, at most two.
    pub band: Vec<WrittenEdge>,
    /// The conditions the band does not hold, in the order written.
    pub rest: Vec<ryo_values::domain::query_syntax::types::Condition>,
}
