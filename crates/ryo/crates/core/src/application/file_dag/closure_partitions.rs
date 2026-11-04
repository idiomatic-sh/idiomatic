//! One prepared shared ordering and one reusable partition-local adjacency.
use super::{derived::view::AcquiredView, relations::RelationOwner};
use crate::domain::{
    derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch, ViewRequest},
    operating::errors::OperatingError,
    operator::contract::views::BorrowedRows,
};
use ryo_derived::domain::contract::types::ClosurePartition;
use ryo_derived::domain::contract::types::{ChainLink, HashedRow, KeyCellsHash};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::{rc::Rc, sync::Arc};

/// Ordering metadata can be shared across consumers; only the current partition
/// owns values/adjacency. Advancing releases the previous partition's logical contents.
pub(crate) struct PartitionedClosure<'f> {
    input: BorrowedRows<'f>,
    owner: Rc<RelationOwner>,
    partition_plan: Arc<DerivedContract>,
    closure_plan: Arc<DerivedContract>,
    ordered: AcquiredView,
    ordering_capacity: usize,
    cursor: usize,
    key: Vec<Value>,
    current: Option<AcquiredView>,
    scratch: DerivedScratch,
}
impl<'f> PartitionedClosure<'f> {
    pub(crate) fn from_prepared(
        input: BorrowedRows<'f>,
        partition_plan: Arc<DerivedContract>,
        closure_plan: Arc<DerivedContract>,
        ordered: AcquiredView,
        owner: Rc<RelationOwner>,
    ) -> Result<Self, OperatingError> {
        crate::domain::derived_views::service::validate(
            ryo_derived::domain::contract::types::DerivedKind::PartitionOrder,
            &partition_plan,
        )?;
        crate::domain::derived_views::service::validate(
            ryo_derived::domain::contract::types::DerivedKind::ClosureAdjacency,
            &closure_plan,
        )?;
        if !std::ptr::eq(ordered.contract(), partition_plan.as_ref())
            || !matches!(
                ordered.view()?,
                ryo_derived::domain::contract::types::DerivedView::PartitionOrder(_)
            )
        {
            return Err(OperatingError::WrongOperation);
        }
        let ordering_capacity = ordered.partition_order_capacity()?;
        Ok(Self {
            input,
            owner,
            partition_plan,
            closure_plan,
            ordered,
            ordering_capacity,
            cursor: 0,
            key: Vec::new(),
            current: None,
            scratch: DerivedScratch::default(),
        })
    }
    #[cfg(test)]
    pub(crate) fn prepare(
        input: BorrowedRows<'f>,
        keys: &[ColumnIndex],
        source: ColumnIndex,
        destination: ColumnIndex,
        owner: Rc<RelationOwner>,
    ) -> Result<Self, OperatingError> {
        input.validate()?;
        let schema = [crate::domain::operation_schema::types::InputSchema {
            kinds: input.kinds(),
            tree: None,
        }];
        let partition_plan = Arc::new(super::derived::prepare::view(
            owner.derived.semantic_port(),
            &ViewRequest::PartitionOrder {
                keys: keys.to_vec(),
            },
            &schema,
        )?);
        let closure_plan = Arc::new(super::derived::prepare::view(
            owner.derived.semantic_port(),
            &ViewRequest::ClosureAdjacency {
                source,
                destination,
            },
            &schema,
        )?);
        let proof = owner.admit_external(input)?;
        let (population, _) = proof.identity(owner.as_ref())?;
        let ordered = owner.derived.acquire_stored(
            &partition_plan,
            &[population],
            super::derived::owner::ViewContext::default(),
            DerivedInputs::Rows(&[proof.rows()]),
            &mut DerivedScratch::default(),
            None,
        )?;
        Self::from_prepared(input, partition_plan, closure_plan, ordered, owner)
    }
    pub(crate) fn advance(&mut self) -> Result<bool, OperatingError> {
        let ryo_derived::domain::contract::types::DerivedView::PartitionOrder(order) =
            self.ordered.view()?
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Some(&first) = order.get(self.cursor) else {
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
        let begin = self.cursor;
        while let Some(&row) = order.get(self.cursor) {
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
            self.cursor += 1;
        }
        // The old partition's mandatory charge ends before its reusable buffer
        // is rebuilt for the next key. A taken partition transfers its lease.
        self.current = Some(self.owner.derived.scoped_build(
            crate::application::file_dag::route_counts::BuildRoute::ClosurePartition,
            &self.closure_plan,
            DerivedInputs::Partition {
                rows: self.input,
                positions: &order[begin..self.cursor],
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
    pub(crate) fn current(&self) -> &ClosurePartition {
        let ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(current) = self
            .current
            .as_ref()
            .expect("advanced current partition")
            .view()
            .expect("validated current partition")
        else {
            unreachable!("prepared closure adjacency")
        };
        current
    }

    /// All live Value payload cells owned by this cursor, not only adjacency rows.
    pub(crate) fn working_cells(&self) -> usize {
        self.current.as_ref().map_or(0, |_| {
            let current = self.current();
            current.table().cells.len() + current.key().len() + current.sources().len()
        }) + self.key.len()
            + self.scratch.row.len()
    }
    /// Reserved Value buffers; shared ordering remains charged independently.
    pub(crate) fn working_capacity(&self) -> usize {
        self.current.as_ref().map_or(0, |_| {
            let current = self.current();
            let (keys, sources) = current.capacities();
            current.table().cells.capacity() + keys + sources
        }) + self.key.capacity()
            + self.scratch.row.capacity()
    }
    pub(crate) fn ordering_bytes(&self) -> usize {
        let mut bytes = self
            .ordering_capacity
            .saturating_mul(size_of::<RowIndex>())
            .saturating_add(self.working_capacity().saturating_mul(size_of::<Value>()));
        if self.current.is_some() {
            let current = self.current();
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
mod tests;

use ryo_values::domain::rows::types::RowIndex;
