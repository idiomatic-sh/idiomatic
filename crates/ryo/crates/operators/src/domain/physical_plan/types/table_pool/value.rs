//! The emptied node tables one worker keeps for its next nodes, so a node's table and its kinds are not allocated again per
//! file; capped in each table's room and in how many it keeps.
use ryo_values::domain::rows::types::{RowCount, Table};

/// A table of more room for cells than this is dropped, not pooled: 4 Ki cells, 64 KiB of 16-byte cells.
pub(crate) const POOL_TABLE_CELLS: usize = 1 << 12;
/// A table of more room for kinds than this is dropped, not pooled.
pub(crate) const POOL_TABLE_KINDS: usize = 64;
/// The most tables a worker's pool keeps.
pub(crate) const POOL_TABLES: usize = 256;

/// The emptied node tables a worker keeps for its next nodes: at most `POOL_TABLES`, each of no more room than
/// `POOL_TABLE_CELLS` cells and `POOL_TABLE_KINDS` kinds, so a worker keeps at most about 16 MiB of them.
#[derive(Debug, Default)]
pub(crate) struct TablePool {
    /// The tables kept, each empty.
    tables: Vec<Table>,
}

/// How many tables a pool keeps.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PooledTables(pub(crate) usize);

impl TablePool {
    /// A table of no rows for a node to write: one the pool kept, or a new one.
    pub(crate) fn fresh(&mut self) -> Table {
        self.tables.pop().unwrap_or_default()
    }

    /// Keeps `table`'s room for a later node, emptied; a table past either cap, or one more than the pool keeps, is dropped.
    pub(crate) fn pooled(&mut self, mut table: Table) {
        if table.cells.capacity() > POOL_TABLE_CELLS
            || table.kinds.capacity() > POOL_TABLE_KINDS
            || self.tables.len() >= POOL_TABLES
        {
            return;
        }
        table.cells.clear();
        table.kinds.clear();
        table.count = RowCount(0);
        self.tables.push(table);
    }

    /// How many tables the pool keeps.
    #[cfg(test)]
    pub(crate) fn kept(&self) -> PooledTables {
        PooledTables(self.tables.len())
    }

    /// Whether every table the pool keeps is empty and within both caps.
    #[cfg(test)]
    pub(crate) fn within_caps(&self) -> crate::domain::contract::types::PredicateOutcome {
        if self.tables.iter().all(|table| {
            table.cells.is_empty()
                && table.count == RowCount(0)
                && table.cells.capacity() <= POOL_TABLE_CELLS
                && table.kinds.capacity() <= POOL_TABLE_KINDS
        }) {
            crate::domain::contract::types::PredicateOutcome::Holds
        } else {
            crate::domain::contract::types::PredicateOutcome::Fails
        }
    }
}
