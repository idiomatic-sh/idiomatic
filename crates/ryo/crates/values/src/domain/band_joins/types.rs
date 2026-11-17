//! The types of the `band_joins` concept: a join whose `where` compares its sides in order, its band and its answer. Plain data,
//! public fields and derives, with no constructor and no inherent impl.

use crate::domain::plan_parts::types::{ColumnIndex, Input, JoinColumn, KeyFan, PlanComparison};

/// How a band edge compares the right column with the left one.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::band_joins::types::BandTest;
/// assert_ne!(BandTest::Below, BandTest::AtOrBelow);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BandTest {
    /// The right value is less than the left one.
    Below,
    /// The right value is at most the left one.
    AtOrBelow,
    /// The right value is greater than the left one.
    Above,
    /// The right value is at least the left one.
    AtOrAbove,
}

/// One comparison of a band join between a left column and a right column: `right test left`.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::band_joins::types::{BandEdge, BandTest};
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// let edge = BandEdge { left: ColumnIndex(0), test: BandTest::Below, right: ColumnIndex(1) };
/// assert_eq!(edge.test, BandTest::Below);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BandEdge {
    /// The left column.
    pub left: ColumnIndex,
    /// How the right value compares with the left one.
    pub test: BandTest,
    /// The right column.
    pub right: ColumnIndex,
}

/// Which right rows of its band a band join answers for each left row.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::band_joins::types::BandAnswer;
/// assert_ne!(BandAnswer::Every, BandAnswer::Nearest);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BandAnswer {
    /// Every right row in the band: `join` and `all-pairs join`.
    Every,
    /// The right rows nearest the left row by the first edge, ties all kept: `nearest join`.
    Nearest,
}

/// A join of rows with equal keys whose columns also stand in order across the sides: each left row meets the right rows of
/// its key inside its band, every one or the nearest.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::band_joins::types::{BandAnswer, BandEdge, BandJoin, BandTest};
/// use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, KeyFan, StepId};
/// let join = BandJoin {
///     left: Input::Step(StepId(0)),
///     right: Input::Step(StepId(1)),
///     left_on: vec![ColumnIndex(0)],
///     right_on: vec![ColumnIndex(0)],
///     band: vec![BandEdge { left: ColumnIndex(1), test: BandTest::Below, right: ColumnIndex(1) }],
///     answer: BandAnswer::Nearest,
///     output: Vec::new(),
///     filter: Vec::new(),
///     view: Vec::new(),
///     fan: KeyFan::Shared,
/// };
/// assert_eq!(join.answer, BandAnswer::Nearest);
/// ```
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct BandJoin {
    /// The left input.
    pub left: Input,
    /// The right input.
    pub right: Input,
    /// The left key columns.
    pub left_on: Vec<ColumnIndex>,
    /// The right key columns.
    pub right_on: Vec<ColumnIndex>,
    /// The band: one or two comparisons of a right column with a left one; the first orders the right rows.
    pub band: Vec<BandEdge>,
    /// Which right rows of the band it answers.
    pub answer: BandAnswer,
    /// The columns of the answer.
    pub output: Vec<JoinColumn>,
    /// What the rows of the right input must satisfy to be joined, as the comparisons of a filter of it.
    pub filter: Vec<PlanComparison>,
    /// The columns of the right input the keys, the band and the answer name, in order; none named means all of them.
    pub view: Vec<ColumnIndex>,
    /// What its keys are proven to pair.
    pub fan: KeyFan,
}
