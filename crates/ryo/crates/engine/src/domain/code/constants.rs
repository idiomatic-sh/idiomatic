//! The constants of the `code` concept: the bits a word's flag holds, by the codes that read them, and the slot that says "none".

use super::types::{WordFlag, WordSlot};

/// A slot that names nothing: no plan point, no ordered column, no site.
pub(crate) const NO_SLOT: WordSlot = WordSlot(u32::MAX);

/// On a node-source code: the file's own tree only, its embedded segments left out.
pub(crate) const OWN_TREE: WordFlag = WordFlag(0x80);
/// `EachKind`: several kinds, listed in the pool.
pub(crate) const SEVERAL: WordFlag = WordFlag(0x01);
/// A match, a `Let`: the value is a constant's number.
pub(crate) const CONSTANT: WordFlag = WordFlag(0x01);
/// `EachMatchMade`: several values, in the pool.
pub(crate) const MANY: WordFlag = WordFlag(0x02);
/// `EachMatchMade`, `EachRowMade`: the table is written, the same for every file.
pub(crate) const WRITTEN: WordFlag = WordFlag(0x04);
/// `EachRange`: the range has a low end.
pub(crate) const LOW_PRESENT: WordFlag = WordFlag(0x01);
/// `EachRange`: a row at the low end is in the range.
pub(crate) const LOW_INCLUSIVE: WordFlag = WordFlag(0x02);
/// `EachRange`: the range has a high end.
pub(crate) const HIGH_PRESENT: WordFlag = WordFlag(0x04);
/// `EachRange`: a row at the high end is in the range.
pub(crate) const HIGH_INCLUSIVE: WordFlag = WordFlag(0x08);
/// `EachRange`: the range column is written to a register.
pub(crate) const RANGE_OUT: WordFlag = WordFlag(0x10);
/// `Nearest`, `NearestDerived`: the answer is kept per start.
pub(crate) const KEPT: WordFlag = WordFlag(0x01);
/// `Test`: the left operand is a constant's number.
pub(crate) const LEFT_CONSTANT: WordFlag = WordFlag(0x01);
/// `Test`: the right operand is a constant's number.
pub(crate) const RIGHT_CONSTANT: WordFlag = WordFlag(0x02);
/// `Test`: the right operand is a sorted list of constants in the pool.
pub(crate) const RIGHT_SET: WordFlag = WordFlag(0x04);
/// `TestInConstants`: the set must not hold the value.
pub(crate) const NEGATED: WordFlag = WordFlag(0x01);
/// `Let`: the value is the scanned file.
pub(crate) const THE_FILE: WordFlag = WordFlag(0x02);
/// `Exists`: the body must have no row.
pub(crate) const ABSENT: WordFlag = WordFlag(0x01);
/// `Exists`, `Or`: the bits the memo's storage is written in, one place up.
pub(crate) const STORAGE: WordFlag = WordFlag(0x06);
/// Under `STORAGE`: the answer is kept once per file, in a slot.
pub(crate) const STORAGE_FILE_SLOT: WordFlag = WordFlag(0x02);
/// Under `STORAGE`: the answer is kept per node, in a slab.
pub(crate) const STORAGE_NODE_SLAB: WordFlag = WordFlag(0x04);
/// Under `STORAGE`: the answer is kept by the values of the key registers, in a map.
pub(crate) const STORAGE_KEYED_MAP: WordFlag = WordFlag(0x06);
/// `Latest`: the bits the bound's side is written in.
pub(crate) const SIDE: WordFlag = WordFlag(0x03);
/// `Latest`: the bound is a constant's number.
pub(crate) const BOUND_CONSTANT: WordFlag = WordFlag(0x04);
/// `Emit`: the row is of the rule's frontier entry.
pub(crate) const FRONTIER: WordFlag = WordFlag(0x01);
/// `StageScan`, `StageProduct`, `StageRead`: the bits the source is written in.
pub(crate) const SOURCE: WordFlag = WordFlag(0x03);
/// `StageScan`: one constant bucket of the source.
pub(crate) const BUCKET: WordFlag = WordFlag(0x04);
/// `StageProbe`, `StageUse`: the bits the mode is written in.
pub(crate) const MODE: WordFlag = WordFlag(0x03);
/// `StageProbe`: the values are operands in the pool.
pub(crate) const PROBE_MANY: WordFlag = WordFlag(0x04);
/// `StageProbe`: a range follows the values.
pub(crate) const PROBE_RANGE: WordFlag = WordFlag(0x08);
/// `StageProbe`: a row at the range's low end is in the range.
pub(crate) const PROBE_LOW_INCLUSIVE: WordFlag = WordFlag(0x10);
/// `StageProbe`: a row at the range's high end is in the range.
pub(crate) const PROBE_HIGH_INCLUSIVE: WordFlag = WordFlag(0x20);
/// `StageWalk`: an axis; without it, the children.
pub(crate) const WALK_AXIS: WordFlag = WordFlag(0x01);
/// `StageWalk`: the bits the direction is written in, one place up.
pub(crate) const WALK_DIRECTION: WordFlag = WordFlag(0x0e);
/// `StageWrite`: a row the table holds is not written again.
pub(crate) const REPEATS_DROPPED: WordFlag = WordFlag(0x01);
