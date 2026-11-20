//! The types of the `statistics` concept: what a sample run measured, the bytes that carry it and its decoded form.

use crate::domain::code::types::{MeasuredForm, UnitKey};
use crate::domain::programs::types::Programs;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Column, Count, KindBits, Relation};

/// The statistics of a rule set as bytes: what a pack's rules ship beside them. No bytes: no statistics.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::statistics::types::StatisticsBytes;
///
/// assert!(StatisticsBytes(&[]).0.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct StatisticsBytes<'a>(pub &'a [u8]);

/// Statistics written by `learned`: what the host keeps beside the rules and gives the next load.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::statistics::types::EncodedStatistics;
///
/// assert!(EncodedStatistics::default().0.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EncodedStatistics(pub Vec<u8>);

/// What a worker counted in the files it scanned: opaque; made by `counted`, added up by `merged`, read by `learned` and
/// `ledger`.
///
/// ```compile_fail,E0603
/// let forged = ryo_engine::domain::statistics::types::PointCounts(Default::default());
/// ```
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::service::merged;
/// use ryo_engine::domain::statistics::types::PointCounts;
///
/// let mut all = PointCounts::default();
/// merged(&mut all, &PointCounts::default());
/// ```
#[derive(Debug, Default)]
pub struct PointCounts(pub(crate) PointCountsEngine);

/// The private body of `PointCounts`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PointCountsEngine {
    /// The files scanned.
    pub(crate) files: Count,
    /// The runs of each plan point, by its number in `ProgramCode.points`.
    pub(crate) points: Vec<Count>,
    /// What each unit was asked and gave, by its place in `ProgramCode.units`.
    pub(crate) units: Vec<UnitCounts>,
    /// The ids of each kind, by kind code, summed over the files.
    pub(crate) kinds: Vec<Count>,
    /// The rows of each size site, by its place in `ProgramCode.sizes`, summed over the files.
    pub(crate) literals: Vec<Count>,
}

/// What a unit was asked and gave in the files a worker scanned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct UnitCounts {
    /// Uses that reached it.
    pub(crate) calls: Count,
    /// Distinct keys it was evaluated for.
    pub(crate) keys: Count,
    /// Rows it gave.
    pub(crate) rows: Count,
    /// For a table: the rows its fill read and wrote and the steps run inside it.
    pub(crate) build: Count,
    /// The thread time spent in it, what it called left out.
    pub(crate) nanos: Count,
    /// The part of that time spent in files where it gave no row.
    pub(crate) idle_nanos: Count,
}

/// One sample run: the programs it ran and what it counted.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_engine::domain::statistics::types::{Measured, PointCounts};
///
/// let (programs, counts) = (Programs::default(), PointCounts::default());
/// let _sample = Measured { programs: &programs, counts: &counts };
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Measured<'a> {
    /// The programs the sample ran.
    pub programs: &'a Programs,
    /// The counts of every worker, merged.
    pub counts: &'a PointCounts,
}

/// Statistics as the lowering and the passes read them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Statistics {
    /// Which sample run wrote them.
    pub(crate) stage: LearnStage,
    /// The sizes the costing reads.
    pub(crate) sizes: MeasuredSizes,
    /// The units, sorted by key.
    pub(crate) units: Vec<MeasuredUnit>,
}

/// Which sample run statistics are of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LearnStage {
    /// The first run: every flip candidate is lowered in its other form.
    #[default]
    First,
    /// The second: every unit is lowered as its verdicts say.
    Second,
}

/// The mean sizes of a file of the sample.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MeasuredSizes {
    /// Ids per kind.
    pub(crate) kinds: Vec<KindSize>,
    /// Rows per value of a column.
    pub(crate) literals: Vec<LiteralSize>,
}

/// The mean ids of a kind in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KindSize {
    /// The kind.
    pub(crate) kind: KindBits,
    /// Its ids in the sample, and the sample's files.
    pub(crate) ids: Count,
}

/// The mean rows of a relation holding a value in a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LiteralSize {
    /// The relation.
    pub(crate) relation: Relation,
    /// The column.
    pub(crate) column: Column,
    /// The value, as a base word.
    pub(crate) value: CellWord,
    /// Its rows in the sample.
    pub(crate) rows: Count,
}

/// What the sample says of a unit's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum OrderVerdict {
    /// The measured order is taken.
    #[default]
    Learned,
    /// The lowering's order is kept: the second run was dearer, or the form kept was not the one last measured.
    Denied,
}

/// Whether the shape of a pattern's plans decides its form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ShapeChoice {
    /// One form is clean and the other scans, or it binds nothing, or it is no pattern.
    #[default]
    Decided,
    /// It binds a column and both forms are clean, or both scan: a flip candidate.
    Open,
}

/// One unit of the statistics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeasuredUnit {
    /// Its name.
    pub(crate) key: UnitKey,
    /// The form its counts were taken in.
    pub(crate) form: MeasuredForm,
    /// Whether the shape rule left its form open.
    pub(crate) shape: ShapeChoice,
    /// Its order.
    pub(crate) order: OrderVerdict,
    /// The runs of each of its plan points, in point order.
    pub(crate) points: Vec<Count>,
    /// What it was asked and gave.
    pub(crate) counts: UnitCounts,
}
