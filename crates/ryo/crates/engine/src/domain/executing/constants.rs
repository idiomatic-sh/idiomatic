//! The constants of the `executing` concept.

use crate::domain::findings::types::InstructionCount;

/// A rule's allowance in a file, as a multiple of the terms of its bound: its candidates, the keys it asked of call
/// patterns, the rows of each source it read and the rows into and out of each table it reached. One constant of the
/// engine: never a setting and never per rule. Thirteen times the mean rows visited per candidate the prototypes measured
/// (scope-engine3 2.5 part 5).
pub(crate) const ALLOWANCE: InstructionCount = InstructionCount(64);

/// How many cells of a key are handed to the fill side from the stack: a longer key is copied to the heap, once per ask.
pub(crate) const KEY_CELLS: usize = 8;
