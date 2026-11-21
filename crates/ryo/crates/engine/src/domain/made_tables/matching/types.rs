//! The types of the `matching` part: the room an ordered match works in, and what it reads of its two inputs.

use crate::domain::code::types::WordSlot;
use crate::domain::indexes::types::IndexArena;
use ryo_language::domain::facts::types::CellWord;
use ryo_operators::domain::ordered_match::types::MatchedPair;
use ryo_relations::domain::relations::types::{Count, Integer};

/// The room an ordered match works in: the index of each side by its keys, ordered; one key's orders on each side; the
/// pairs of one key; and one key's cells. Kept between fills, never freed.
#[derive(Debug, Default)]
pub(crate) struct MatchRoom {
    /// The index of the left input.
    pub(crate) left: IndexArena,
    /// The index of the right input.
    pub(crate) right: IndexArena,
    /// One key's left orders, ascending.
    pub(crate) left_orders: Vec<Integer>,
    /// One key's right orders, ascending.
    pub(crate) right_orders: Vec<Integer>,
    /// The pairs of one key.
    pub(crate) pairs: Vec<MatchedPair>,
    /// One key's cells.
    pub(crate) key: Vec<CellWord>,
}

/// One side of an ordered match: its key columns and its order column.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MatchSide<'c> {
    /// The key columns.
    pub(crate) keys: &'c [WordSlot],
    /// The order column.
    pub(crate) order: Count,
}
