//! Immutable proofs requested by a recognized ordered-existence physical strategy.
use crate::domain::operator::types::PlanIdentity;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Extremum {
    Maximum,
    Minimum,
}

/// A stored numeric decoder required by the original, uncanonicalized column lineage.
/// This is a requirement, never a caller assertion that runtime admission succeeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StoredNumberOrigin {
    pub relation: Relation,
    pub column: ColumnIndex,
}

/// How many columns the rows of the source hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SchemaWidth(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NumberRequirement {
    pub source: StepId,
    pub column: ColumnIndex,
    pub width: SchemaWidth,
    pub origins: Vec<StoredNumberOrigin>,
}

/// The scheduler must demand S and omit J/F, while retaining the original P and antijoin.
/// No kernel may execute this descriptor until the existing owner binds NumberRequirement
/// to its actual admitted population, schema and generation.
#[derive(Clone, Debug)]
pub(crate) struct Shape {
    pub plan: PlanIdentity,
    pub source: StepId,
    pub replaced_join: StepId,
    pub replaced_select: StepId,
    pub retained_project: StepId,
    pub retained_antijoin: StepId,
    pub partition: Vec<ColumnIndex>,
    pub order: ColumnIndex,
    pub extremum: Extremum,
    pub projected_left: Vec<ColumnIndex>,
    pub number: NumberRequirement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decline {
    NotAntijoin,
    NotDistinctProject,
    NotSinglePredicate,
    NotPlainSelfJoin,
    DifferentSource,
    DifferentPartitionColumns,
    NonStrictOrder,
    DifferentOrderColumns,
    RightProjection,
    ExtraConsumer,
    CrossFile,
    InvalidColumn,
    NotNumber,
    UnprovenNumberOrigin,
}

/// A step the recognizer declined and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Declined {
    pub step: StepId,
    pub reason: Decline,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Program {
    pub shapes: Vec<Shape>,
    pub declines: Vec<Declined>,
}
