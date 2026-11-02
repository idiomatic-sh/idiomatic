//! One reusable adjacency at a time for a borrowed partitioned closure input.
//! The source stays with its existing owner; only ordered row IDs scale with input rows.

use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{
    graph::ClosurePartition, partition::ordered_positions, views::BorrowedRows,
};
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::{
    service::{check_columns, index, push_row},
    types::{ChainLink, HashedRow, KeyCellsHash, KeyIndex, On, Table},
};
use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// A file-local ordered cursor. The current partition reuses its Table, index,
/// key and source buffers; no vector of all partition adjacencies is built.
pub(crate) struct PartitionedClosure<'f> {
    input: BorrowedRows<'f>,
    keys: Vec<ColumnIndex>,
    source: ColumnIndex,
    destination: ColumnIndex,
    ordered: Vec<RowIndex>,
    cursor: usize,
    current: ClosurePartition,
}

impl<'f> PartitionedClosure<'f> {
    pub(crate) fn prepare(
        input: BorrowedRows<'f>,
        keys: &[ColumnIndex],
        source: ColumnIndex,
        destination: ColumnIndex,
    ) -> Result<Self, OperatingError> {
        input.validate()?;
        check_columns(input.kinds(), &[source, destination])?;
        check_columns(input.kinds(), keys)?;
        let ordered = ordered_positions(input, keys)?;
        let endpoint_kinds = [
            *input
                .kinds()
                .get(source.0 as usize)
                .ok_or(OperatingError::ShapeMismatch)?,
            *input
                .kinds()
                .get(destination.0 as usize)
                .ok_or(OperatingError::ShapeMismatch)?,
        ];
        Ok(Self {
            input,
            keys: keys.to_vec(),
            source,
            destination,
            ordered,
            cursor: 0,
            current: ClosurePartition {
                key: Vec::with_capacity(keys.len()),
                table: Table {
                    kinds: endpoint_kinds.to_vec(),
                    ..Table::default()
                },
                index: KeyIndex::default(),
                sources: Vec::new(),
            },
        })
    }

    /// Build only the next exact-key adjacency before any result from it emits.
    /// A false result means the current value should no longer be read.
    pub(crate) fn advance(&mut self) -> Result<bool, OperatingError> {
        let Some(&first) = self.ordered.get(self.cursor) else {
            return Ok(false);
        };
        self.current.key.clear();
        for column in &self.keys {
            self.current.key.push(
                self.input
                    .cell(first, *column)
                    .ok_or(OperatingError::ShapeMismatch)?,
            );
        }
        self.current.table.cells.clear();
        self.current.table.count.0 = 0;
        self.current.sources.clear();
        while let Some(&row) = self.ordered.get(self.cursor) {
            if !self
                .keys
                .iter()
                .zip(&self.current.key)
                .all(|(column, expected)| self.input.cell(row, *column) == Some(*expected))
            {
                break;
            }
            let from = self
                .input
                .cell(row, self.source)
                .ok_or(OperatingError::ShapeMismatch)?;
            let to = self
                .input
                .cell(row, self.destination)
                .ok_or(OperatingError::ShapeMismatch)?;
            push_row(&mut self.current.table, &[from, to]);
            self.current.sources.push(from);
            self.cursor += 1;
        }
        index(
            &self.current.table,
            On::Columns(&[ColumnIndex(0)]),
            &mut self.current.index,
        )?;
        self.current.sources.sort_unstable();
        self.current.sources.dedup();
        Ok(true)
    }

    pub(crate) fn current(&self) -> &ClosurePartition {
        &self.current
    }

    /// All live Value payload cells owned by this cursor, not only adjacency rows.
    pub(crate) fn working_cells(&self) -> usize {
        self.current.table.cells.len() + self.current.key.len() + self.current.sources.len()
    }

    /// All reserved Value slots, including the reusable key and source buffers.
    pub(crate) fn working_capacity(&self) -> usize {
        self.current.table.cells.capacity()
            + self.current.key.capacity()
            + self.current.sources.capacity()
    }

    /// Retained capacity accounted across the entire cursor: row IDs, key
    /// metadata, current adjacency/index, and Value payload. This is capacity
    /// accounting, not allocator headers or process RSS; source rows are borrowed.
    pub(crate) fn ordering_bytes(&self) -> usize {
        fn vector<T>(capacity: usize) -> usize {
            capacity.saturating_mul(size_of::<T>())
        }
        let index = &self.current.index;
        vector::<RowIndex>(self.ordered.capacity())
            .saturating_add(vector::<ColumnIndex>(self.keys.capacity()))
            .saturating_add(vector::<Value>(self.working_capacity()))
            .saturating_add(vector::<CellKind>(self.current.table.kinds.capacity()))
            .saturating_add(vector::<RowIndex>(index.rows.capacity()))
            .saturating_add(vector::<KeyCellsHash>(index.hashes.capacity()))
            .saturating_add(vector::<ChainLink>(index.heads.capacity()))
            .saturating_add(vector::<ChainLink>(index.next.capacity()))
            .saturating_add(vector::<RowIndex>(index.sorted.capacity()))
            .saturating_add(vector::<HashedRow>(index.pairs.capacity()))
            .saturating_add(
                index
                    .runs
                    .capacity()
                    .saturating_mul(size_of::<KeyCellsHash>() + 1),
            )
    }
}

#[cfg(test)]
mod tests;
