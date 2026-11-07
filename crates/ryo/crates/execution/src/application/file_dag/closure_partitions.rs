//! One prepared shared ordering and one reusable partition-local adjacency.
use super::{
    derived::owner::acquisition::admission::service::AcquiredView, relations::RelationOwner,
};
use crate::domain::{
    derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch, ViewRequest},
    operating::errors::OperatingError,
    operator::contract::views::BorrowedRows,
};
use ryo_derived::domain::contract::types::ClosurePartition;
use ryo_derived::domain::contract::types::{ChainLink, HashedRow, KeyCellsHash};
use ryo_relations::domain::relations::types::{CellKind, Value};
use std::{rc::Rc, sync::Arc};

/// Ordering metadata can be shared across consumers; only the current partition
/// owns values/adjacency. Advancing releases the previous partition's logical contents.
pub(crate) struct PartitionedClosure<'f> {
    input: BorrowedRows<'f>,
    proof: super::relations::admission::service::BorrowedInput<'f>,
    owner: Rc<RelationOwner>,
    partition_plan: Arc<DerivedContract>,
    closure_plan: Arc<DerivedContract>,
    ordered: AcquiredView,
    #[cfg(test)]
    ordering_capacity: crate::application::file_dag::derived::owner::acquisition::admission::types::PartitionCapacity,
    cursor: OrderCursor,
    key: Vec<Value>,
    current: Option<AcquiredView>,
    scratch: DerivedScratch,
}
impl<'f> PartitionedClosure<'f> {
    pub(crate) fn advance(&mut self) -> Result<bool, OperatingError> {
        let ryo_derived::domain::contract::types::DerivedView::PartitionOrder(order) =
            self.ordered.view()?
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Some(&first) = order.get(self.cursor.0) else {
            self.current = None;
            return Ok(false);
        };
        let ViewRequest::PartitionOrder { keys } = &self.partition_plan.request else {
            return Err(OperatingError::WrongOperation);
        };
        self.key.clear();
        for column in keys {
            self.key.push(
                self.input
                    .cell(first, *column)
                    .ok_or(OperatingError::ShapeMismatch)?,
            );
        }
        let begin = self.cursor.0;
        while let Some(&row) = order.get(self.cursor.0) {
            let same_key = keys
                .iter()
                .zip(&self.key)
                .try_fold(true, |same, (column, value)| {
                    let actual = self
                        .input
                        .cell(row, *column)
                        .ok_or(OperatingError::ShapeMismatch)?;
                    Ok::<_, OperatingError>(same && actual == *value)
                })?;
            if !same_key {
                break;
            }
            self.cursor = OrderCursor(self.cursor.0 + 1);
        }
        // The old partition's mandatory charge ends before its reusable buffer
        // is rebuilt for the next key. A taken partition transfers its lease.
        let admitted = self.owner.derived().admit_partition(
            &self.closure_plan,
            &self.owner,
            &self.proof,
            &self.ordered,
        )?;
        self.current = Some(self.owner.derived().scoped_build(
            crate::application::file_dag::route_counts::types::BuildRoute::ClosurePartition,
            &self.closure_plan,
            &admitted,
            DerivedInputs::Partition {
                rows: self.input,
                positions: &order[begin..self.cursor.0],
                key: &self.key,
            },
            self.current.take(),
            &mut self.scratch,
        )?);
        Ok(true)
    }
    pub(in crate::application::file_dag) fn take_current(
        &mut self,
    ) -> Result<AcquiredView, OperatingError> {
        self.current.take().ok_or(OperatingError::NotRun)
    }
    pub(crate) fn current(&self) -> Result<&ClosurePartition, OperatingError> {
        let current = self.current.as_ref().ok_or(OperatingError::NotRun)?;
        match current.view()? {
            ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(partition) => {
                Ok(partition)
            }
            _ => Err(OperatingError::WrongOperation),
        }
    }

    /// All live Value payload cells owned by this cursor, not only adjacency rows.
    #[cfg(test)]
    pub(crate) fn working_cells(&self) -> usize {
        self.current.as_ref().map_or(0, |_| {
            let current = self.current().expect(constants::LIVE_PARTITION_VIEW);
            current.table().cells.len() + current.key().len() + current.sources().len()
        }) + self.key.len()
            + self.scratch.row.len()
    }
    /// Reserved Value buffers; shared ordering remains charged independently.
    #[cfg(test)]
    pub(crate) fn working_capacity(&self) -> usize {
        self.current.as_ref().map_or(0, |_| {
            let current = self.current().expect(constants::LIVE_PARTITION_VIEW);
            let (keys, sources) = current.capacities();
            current.table().cells.capacity() + keys + sources
        }) + self.key.capacity()
            + self.scratch.row.capacity()
    }
    #[cfg(test)]
    pub(crate) fn ordering_bytes(&self) -> usize {
        let mut bytes = self
            .ordering_capacity
            .0
            .saturating_mul(size_of::<RowIndex>())
            .saturating_add(self.working_capacity().saturating_mul(size_of::<Value>()));
        if self.current.is_some() {
            let current = self.current().expect(constants::LIVE_PARTITION_VIEW);
            let [rows, hashes, heads, links, sorted, pairs, runs] = current.index().capacities();
            bytes = bytes
                .saturating_add(current.table().kinds.capacity() * size_of::<CellKind>())
                .saturating_add((rows + sorted) * size_of::<RowIndex>())
                .saturating_add(hashes * size_of::<KeyCellsHash>())
                .saturating_add((heads + links) * size_of::<ChainLink>())
                .saturating_add(pairs * size_of::<HashedRow>())
                .saturating_add(runs.saturating_mul(size_of::<KeyCellsHash>() + 1));
        }
        bytes
    }
}

#[cfg(test)]
mod partitions_test;
mod constants;
pub(in crate::application::file_dag) mod schemas;
mod types;
use types::OrderCursor;

use ryo_values::domain::rows::types::RowIndex;
