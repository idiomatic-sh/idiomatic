//! The texts of the `positions` concept.

/// What a read of a row of the positions says when it is given a row past the last.
pub(super) const ROW_OF_POSITIONS: &str = "a row these positions hold";

/// What a read of a row says when a column's slot is past the row.
pub(super) const SLOT_OF_ROW: &str = "a slot of the row its column was lowered for";

/// What a read of a step's columns says when a key names a column past them.
pub(super) const COLUMN_OF_STEP: &str = "a column of the step the key was lowered for";

/// What a join or a walk says when a row's position is past the tree it reads.
pub(super) const POSITION_OF_TREE: &str = "a position of the tree the rows were read from";
