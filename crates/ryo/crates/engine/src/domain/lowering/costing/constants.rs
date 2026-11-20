//! The constants of the `costing` part: the fixed priors a load with no statistics orders accesses by (scope-engine3 2.2 row
//! 18). Each is an estimate of rows in one file; none is a bound, and none refuses anything.

use crate::domain::lowering::definitions::types::RowEstimate;

/// The elements of a file's node table.
pub(crate) const ELEMENTS: RowEstimate = RowEstimate(2_048);
/// The rows of a filled relation.
pub(crate) const FILLED_ROWS: RowEstimate = RowEstimate(512);
/// The rows of a group table.
pub(crate) const GROUP_ROWS: RowEstimate = RowEstimate(64);
/// A literal with no counts is sized at one row in this many of its source.
pub(crate) const LITERAL_SHARE: u64 = 256;
/// A kind's ids are one element in this many.
pub(crate) const KIND_SHARE: u64 = 32;
/// A key that is not the source's own reaches one row in this many of it.
pub(crate) const KEY_SHARE: u64 = 64;
/// The children of one parent.
pub(crate) const CHILDREN: RowEstimate = RowEstimate(4);
/// The rows of a relation's own key that holds any number of rows per value.
pub(crate) const MANY_ROWS: RowEstimate = RowEstimate(2);
/// The ancestors of one element.
pub(crate) const ANCESTORS: RowEstimate = RowEstimate(8);
/// The descendants of one element.
pub(crate) const DESCENDANTS: RowEstimate = RowEstimate(32);
/// The anchors tried for a branch that has no given cell: the cheapest of this many.
pub(crate) const ANCHORS_TRIED: usize = 4;
/// The greatest estimate: every estimate past it is it.
pub(crate) const ESTIMATE_CAP: RowEstimate = RowEstimate(1 << 40);
