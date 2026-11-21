//! The constants of the `indexes` concept.

use super::types::{HashShift, RowHash, TableWord};
use ryo_relations::domain::relations::types::Count;

/// The run of an entry no key has taken.
pub(crate) const EMPTY_RUN: TableWord = TableWord(u64::MAX);

/// What a key's cells are multiplied by, one after the other, to spread them over a table: the odd number nearest to 2^64
/// over the golden ratio.
pub(crate) const HASH_MULTIPLIER: RowHash = RowHash(0x9e37_79b9_7f4a_7c15);

/// How far a hash is turned before the next cell of a key is mixed in.
pub(crate) const HASH_TURN: HashShift = HashShift(26);

/// The fewest entries a table has: a table of one entry would shift a hash by its whole width.
pub(crate) const LEAST_ENTRIES: Count = Count(8);

/// The low half of a run: where it ends.
pub(crate) const RUN_END: TableWord = TableWord(0xffff_ffff);

/// How far a run's start is shifted.
pub(crate) const RUN_START_SHIFT: HashShift = HashShift(32);

/// How many times its rows the cells of a one-column key may span for its entries to be placed by the cell itself.
pub(crate) const PLACED_SPAN: Count = Count(2);

/// How far a hash is folded onto itself for a map that reads its low bits: the keys asked of a relation, the rows a
/// table holds.
pub(crate) const HASH_FOLD: HashShift = HashShift(32);
