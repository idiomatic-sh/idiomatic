//! Immutable proofs requested by a recognized ordered-existence physical strategy.
use crate::domain::operator::types::PlanIdentity;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};

/// Which end of an order a self join keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extremum {
    /// The greatest.
    Maximum,
    /// The least.
    Minimum,
}

/// A stored numeric decoder required by the original, uncanonicalized column lineage.
/// This is a requirement, never a caller assertion that runtime admission succeeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredNumberOrigin {
    /// The relation the number is stored in.
    pub relation: Relation,
    /// The column the number is stored in.
    pub column: ColumnIndex,
}

/// How many columns the rows of the source hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchemaWidth(pub usize);

/// The number the rows of an ordered existence must carry: the step and column it is read from, the schema width and where it is stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberRequirement {
    /// The step the number is read from.
    pub source: StepId,
    /// The column it is read from.
    pub column: ColumnIndex,
    /// The width of the step's schema.
    pub width: SchemaWidth,
    /// Where the number is stored.
    pub origins: Vec<StoredNumberOrigin>,
}

/// The scheduler must demand S and omit J/F, while retaining the original P and antijoin.
/// No kernel may execute this descriptor until the existing owner binds NumberRequirement
/// to its actual admitted population, schema and generation.
#[derive(Clone, Debug)]
pub struct Shape {
    /// The identity of the plan the shape was recognised in.
    pub plan: PlanIdentity,
    /// The step whose rows the shape reads.
    pub source: StepId,
    /// The join the shape replaces.
    pub replaced_join: StepId,
    /// The select the shape replaces.
    pub replaced_select: StepId,
    /// The projection the shape keeps.
    pub retained_project: StepId,
    /// The antijoin the shape keeps.
    pub retained_antijoin: StepId,
    /// The columns the rows are partitioned by.
    pub partition: Vec<ColumnIndex>,
    /// The column that orders a partition.
    pub order: ColumnIndex,
    /// Which end of the order is kept.
    pub extremum: Extremum,
    /// The left columns the projection keeps.
    pub projected_left: Vec<ColumnIndex>,
    /// The number the rows must carry.
    pub number: NumberRequirement,
}

/// Why the recogniser declined a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decline {
    /// The step is not an antijoin.
    NotAntijoin,
    /// Its input is not a distinct projection.
    NotDistinctProject,
    /// Its select has more than one predicate.
    NotSinglePredicate,
    /// The join is not a plain self join.
    NotPlainSelfJoin,
    /// The two sides read different sources.
    DifferentSource,
    /// The two sides partition by different columns.
    DifferentPartitionColumns,
    /// The order is not strict.
    NonStrictOrder,
    /// The two sides order by different columns.
    DifferentOrderColumns,
    /// The right side is projected.
    RightProjection,
    /// Another step consumes an intermediate result.
    ExtraConsumer,
    /// The shape reads rows across files.
    CrossFile,
    /// A column is out of range.
    InvalidColumn,
    /// The order column is not a number.
    NotNumber,
    /// The number's stored origin is not proven.
    UnprovenNumberOrigin,
}

/// A step the recognizer declined and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Declined {
    /// The declined step.
    pub step: StepId,
    /// Why it was declined.
    pub reason: Decline,
}

/// What the recogniser found in a plan: the shapes it recognised and the steps it declined.
#[derive(Clone, Debug, Default)]
pub struct Program {
    /// The recognised shapes.
    pub shapes: Vec<Shape>,
    /// The declined steps.
    pub declines: Vec<Declined>,
}
