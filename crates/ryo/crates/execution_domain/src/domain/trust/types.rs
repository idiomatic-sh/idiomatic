//! The types of the `trust` concept: plain data, public fields and derives, with no constructor and no inherent impl.

pub mod relation_read;

use relation_read::value::RelationRead;

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
