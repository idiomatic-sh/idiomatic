//! What a band join lowers to.
use ryo_values::domain::plan_parts::types::{
    BandAnswer, BandEdge, ColumnIndex, JoinColumn, KeyFan,
};

/// What a band join reads and answers, every right column named as the right rows hold it (its view applied).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BandParts {
    /// The left key columns.
    pub(crate) left_on: Vec<ColumnIndex>,
    /// The right key columns.
    pub(crate) right_on: Vec<ColumnIndex>,
    /// The band: one or two comparisons of a right column with a left one, the first ordering the right rows.
    pub(crate) band: Vec<BandEdge>,
    /// Which right rows of the band it answers.
    pub(crate) answer: BandAnswer,
    /// The columns of the answer.
    pub(crate) output: Vec<JoinColumn>,
    /// What its keys are proven to pair.
    pub(crate) fan: KeyFan,
}
