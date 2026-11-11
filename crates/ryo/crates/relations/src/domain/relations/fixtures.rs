//! The test support of the `relations` concept: the catalog lookups the tests of other crates make.

use super::service;
use crate::domain::relations::types::{Bits, CellKind, FileId, NodeId, Position, Relation, Value};

/// The cell kinds of a relation.
#[must_use]
pub fn kinds(relation: Relation) -> &'static [CellKind] {
    service::kinds(relation)
}

/// The id of the node at a position of a file.
#[must_use]
pub fn node_id(file: FileId, position: Position) -> NodeId {
    service::node_id(file, position)
}

/// The bits a value is stored as.
#[must_use]
pub fn to_bits(value: Value) -> Bits {
    service::to_bits(value)
}
