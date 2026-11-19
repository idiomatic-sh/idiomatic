//! Borrowed ordered match contracts; no engine authority.

/// What an ordered match lowers to: both sides' keys and orders and the output columns.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OrderedParts {
    /// The left keys.
    pub(crate) left_on: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The right keys.
    pub(crate) right_on: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The left order column.
    pub(crate) left_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The right order column.
    pub(crate) right_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The output columns.
    pub(crate) output: Vec<ryo_values::domain::plan_parts::types::JoinColumn>,
    /// Spike (ryo-fmx3): when set, a band semi match instead of the FIFO pairing (`band_existence`).
    pub(crate) band: Option<Band>,
}

/// Spike (ryo-fmx3): a band semi match: each left row, in its order, whose equal-keyed right rows hold one whose order lies
/// within every bound the left row's own columns set; its output columns (all of the left side) once, a repeat dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Band {
    /// The bounds the right order must pass.
    pub(crate) bounds: Vec<BandBound>,
}

/// Spike (ryo-fmx3): one bound of a band: the right order against a left column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BandBound {
    /// The left column.
    pub(crate) column: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// Whether the right order lies above it (a lower bound) or below it.
    pub(crate) above: bool,
    /// Whether equal passes.
    pub(crate) inclusive: bool,
}
