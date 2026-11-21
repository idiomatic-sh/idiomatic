//! The constants of the `executing` concept.

use crate::domain::findings::types::InstructionCount;
use crate::domain::memos::types::MemoId;
use ryo_relations::domain::relations::types::Count;

/// A rule's allowance in a file, as a multiple of the terms of its bound: its candidates, the keys it asked of call
/// patterns, the rows of each source it read and the rows into and out of each table it reached. One constant of the
/// engine: never a setting and never per rule. Thirteen times the mean rows visited per candidate the prototypes measured
/// (scope-engine3 2.5 part 5).
pub(crate) const ALLOWANCE: InstructionCount = InstructionCount(64);

/// The rows a relation that grows with the file's elements may hold for each element of the file, before the engine says
/// it is oversized (scope-engine3 2.5 part 7b).
pub(crate) const ROWS_PER_ELEMENT: Count = Count(16);

/// The rows such a relation may hold beside: a small file's relation is never oversized.
pub(crate) const ROWS_BESIDE: Count = Count(256);

/// The answer of an ask that was not made: the pattern's definition cannot have a row in the file. It names no kept
/// answer, so it has no row and no charge (`memos::service::{rows, row_count}` answer nothing for it).
pub(crate) const NO_ANSWER: MemoId = MemoId(u32::MAX);
