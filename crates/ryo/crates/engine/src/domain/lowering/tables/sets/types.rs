//! The types of the `sets` part.

use crate::domain::lowering::flattening::types::FlatTerm;
use ryo_relations::domain::relations::types::{Column, Relation};

/// How a selection takes a comparison that holds a node's kind or field to a set of literals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetTests {
    /// One branch for each literal, each a pin of the cell: the union of the literals' buckets.
    Buckets,
    /// One test of the cell's text, as the plan writes it: where what reads the rows takes one branch alone.
    Tests,
}

/// A cell of a branch that is a node's kind or field: the term, and the column of the node relation it is the cell of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BucketCell {
    /// The cell's term.
    pub(crate) term: FlatTerm,
    /// The node relation whose row holds the cell.
    pub(crate) relation: Relation,
    /// The relation's column: its kind or its field.
    pub(crate) column: Column,
}

/// A column of a relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RelationColumn {
    /// The relation.
    pub(crate) relation: Relation,
    /// Its column.
    pub(crate) column: Column,
}
