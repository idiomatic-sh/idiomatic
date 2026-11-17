//! Borrowed equality join contracts, and what a band join of keys and order comparisons lowers to; no engine authority.
use crate::domain::select::types::SelectParts;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, RepeatedRows};

/// What a join reads from its operation: its key columns, its output, the declared right view, and the
/// reachability lookup of a logical closure join.
#[derive(Clone, Copy, Debug)]
pub(crate) struct JoinShape<'a> {
    /// The left key columns.
    pub left_on: &'a [ColumnIndex],
    /// The right key columns.
    pub right_on: &'a [ColumnIndex],
    /// The output columns.
    pub output: &'a [JoinColumn],
    /// The logical-to-physical right column remapping.
    pub view: &'a [ColumnIndex],
    /// The reachability lookup of a join over a closure.
    pub reach: Option<&'a ryo_values::domain::plan_parts::types::ReachLookup>,
    /// What the keys are proven to pair.
    pub fan: ryo_values::domain::plan_parts::types::KeyFan,
}

/// Which left rows a probe keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProbeMode {
    /// Each left row with each right row of its key, the right rows ascending; the keys proven to pair as the fan says.
    Inner(ryo_values::domain::plan_parts::types::KeyFan),
    /// A left row with a right row of its key.
    Semi,
    /// A left row with no right row of its key.
    Anti,
    /// A left row with no right row of its key strictly past it in an order: `ordered_existence`'s strategy for an antijoin
    /// of a self join, deciding by each key's extremum instead of pairing rows.
    Ordered(OrderedProbe),
}

#[cfg(test)]
kinds! {
    /// The kind of a `ProbeMode`, without its order: what a test that must see every mode compares against.
    pub(crate) enum ProbeModeKind {
        /// `ProbeMode::Inner`.
        Inner,
        /// `ProbeMode::Semi`.
        Semi,
        /// `ProbeMode::Anti`.
        Anti,
        /// `ProbeMode::Ordered`.
        Ordered,
    }
}

/// Which end of an order an ordered probe keeps. `pub` until F3 commit 4: the engine's runtime names it.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::equality_join::types::PartitionExtremum;
///
/// assert_ne!(PartitionExtremum::Maximum, PartitionExtremum::Minimum);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartitionExtremum {
    /// The greatest: a left row is kept when no right row of its key orders after it.
    Maximum,
    /// The least: a left row is kept when no right row of its key orders before it.
    Minimum,
}

/// The order an ordered probe compares by: a numeric column of each side, and the end it keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OrderedProbe {
    /// The order column of a left row.
    pub(crate) left_order: ColumnIndex,
    /// The order column of a right row.
    pub(crate) right_order: ColumnIndex,
    /// The end it keeps.
    pub(crate) extremum: PartitionExtremum,
}

/// The join a seeded closure answers: the left rows' probe of the reached rows, as the logical join over the closure.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SeededJoin {
    /// The probe of the reached rows by the left rows: its keys, its output and its remaining comparisons.
    pub(crate) probe: ProbeParts,
    /// The logical right row the probe reads, over the partition's keys, then the source, then the vertex reached.
    pub(crate) columns: Vec<ColumnIndex>,
}

/// How a semi or anti probe decides that a key has a right row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Existence {
    /// By reading every right row of the key.
    Every,
    /// At the first right row of the key: the `existence_probe` rewrite.
    First,
}

/// What a probe reads and keeps.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProbeParts {
    /// Which left rows it keeps.
    pub(crate) mode: ProbeMode,
    /// The key columns of each side.
    pub(crate) key: ProbeKey,
    /// The comparisons a right row must pass to be indexed; none keeps every row.
    pub(crate) filter: SelectParts,
    /// The columns of an inner probe's answer, a right column as the right rows hold it; a semi or anti probe answers the
    /// left row.
    pub(crate) output: Vec<JoinColumn>,
    /// How a semi or anti probe decides that a key has a right row.
    pub(crate) existence: Existence,
    /// Whether a repeated answer is kept once.
    pub(crate) repeated: RepeatedRows,
    /// Whether an anti probe of no right rows keeps every left row without reading its key.
    pub(crate) empty_right: EmptyRight,
}

/// What a probe of no right rows answers before it reads a left row's key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmptyRight {
    /// The key is read as for any right rows.
    KeyRead,
    /// An anti probe keeps every left row unread: an antijoin of nothing.
    EveryLeftRow,
}

/// The columns a probe keys each side's rows by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProbeKey {
    /// The left key columns and the right ones, as the right rows hold them.
    Columns {
        /// The left key columns.
        left: Vec<ColumnIndex>,
        /// The right key columns.
        right: Vec<ColumnIndex>,
    },
    /// Every column of each side: a row is its key.
    Whole,
}

/// What a band join reads and answers, every right column named as the right rows hold it (its view applied).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BandParts {
    /// The left key columns.
    pub(crate) left_on: Vec<ColumnIndex>,
    /// The right key columns.
    pub(crate) right_on: Vec<ColumnIndex>,
    /// The band: one or two comparisons of a right column with a left one, the first ordering the right rows.
    pub(crate) band: Vec<ryo_values::domain::band_joins::types::BandEdge>,
    /// Which right rows of the band it answers.
    pub(crate) answer: ryo_values::domain::band_joins::types::BandAnswer,
    /// The columns of the answer.
    pub(crate) output: Vec<JoinColumn>,
    /// What its keys are proven to pair.
    pub(crate) fan: ryo_values::domain::plan_parts::types::KeyFan,
}
