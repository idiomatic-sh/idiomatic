//! The types of the `trust` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_relations::domain::relations::types::Relation;

use ryo_values::domain::plan_parts::types::PlanComparison;

/// A place where a plan reads a relation: the relation, and the comparisons on its `kind` column that the rows must satisfy
/// to get any further, when the step that reads it is a `select`.
#[derive(Clone, Debug, PartialEq)]
pub struct RelationRead {
    /// The relation read.
    pub relation: Relation,
    /// The comparisons on the kind of the rows that this read applies before anything else does.
    pub kinds: Vec<PlanComparison>,
}

/// What the steps of a rule read of the relations, by where the steps run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Needs {
    /// Every read of a relation by the plan: a partly read one makes the answer incomplete, whether or not a step trusts its
    /// absence of rows.
    pub reads: Vec<RelationRead>,
    /// The reads that the steps that run on each file trust the absence of rows of: the file is the whole of what could be missing.
    pub per_file: Vec<RelationRead>,
    /// The reads that the steps that run after the files were read trust the absence of rows of: any file could be missing rows.
    pub cross: Vec<RelationRead>,
}

/// Whether a comparison lets a row of a kind through, as far as it can be told from the comparison alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Passage {
    /// It lets the row through, or cannot tell.
    LetsThrough,
    /// It stops the row.
    Blocks,
}

/// Whether a partly read relation could change what a read of it makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bearing {
    /// It could.
    Matters,
    /// It could not.
    DoesNotMatter,
}

/// Whether any of some reads is of a relation read in part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocking {
    /// One is.
    Blocked,
    /// None is.
    Free,
}

/// Which of the reads of a step one of its columns is the kind of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReadIndex(pub usize);

/// What one step reads of the relations, through the steps before it, and for each of its columns the read whose `kind` column it still is.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StepReads {
    /// The reads of the relations that feed the step.
    pub uses: Vec<RelationRead>,
    /// For each column of the step, the read whose kind it is, when it is one.
    pub provenance: Vec<Option<ReadIndex>>,
}
