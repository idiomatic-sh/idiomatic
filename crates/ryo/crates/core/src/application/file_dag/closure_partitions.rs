//! One prepared shared ordering and one reusable partition-local adjacency.
use super::{
    derived::{ledger::ResourceLease, view::AcquiredView},
    relations::RelationOwner,
};
use crate::domain::{
    derived_views::types::{
        DerivedContract, DerivedInputs, DerivedScratch, DerivedStorage, ViewRequest,
    },
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
    current: DerivedStorage,
    current_lease: Option<Arc<ResourceLease>>,
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
                crate::domain::derived_views::types::DerivedView::PartitionOrder(_)
            )
        {
            return Err(OperatingError::WrongOperation);
        }
        let ordering_capacity = ordered.partition_order_capacity()?;
        let ViewRequest::ClosureAdjacency {
            source,
            destination,
        } = &closure_plan.request
        else {
            return Err(OperatingError::WrongOperation);
        };
        let schema = closure_plan
            .schemas
            .first()
            .ok_or(OperatingError::MissingInput { position: 1 })?;
        let current = DerivedStorage::ClosureAdjacency(ClosurePartition {
            key: Vec::new(),
            sources: Vec::new(),
            index: Default::default(),
            table: ryo_values::domain::rows::types::Table {
                kinds: vec![schema[source.0 as usize], schema[destination.0 as usize]],
                ..Default::default()
            },
        });
        Ok(Self {
            input,
            owner,
            partition_plan,
            closure_plan,
            ordered,
            ordering_capacity,
            cursor: 0,
            key: Vec::new(),
            current,
            current_lease: None,
            scratch: DerivedScratch::default(),
        })
    }
    #[cfg(test)]
    pub(crate) fn prepare(
        input: BorrowedRows<'f>,
        keys: &[ColumnIndex],
        source: ColumnIndex,
        destination: ColumnIndex,
    ) -> Result<Self, OperatingError> {
        use crate::domain::derived_views::collections::views::registration::registered;
        use ryo_derived::domain::contract::types::DerivedKind;
        input.validate()?;
        let schema = [crate::domain::operation_schema::types::InputSchema {
            kinds: input.kinds(),
            tree: None,
        }];
        let partition_plan = Arc::new(registered(DerivedKind::PartitionOrder).prepare(
            &ViewRequest::PartitionOrder {
                keys: keys.to_vec(),
            },
            &schema,
        )?);
        let closure_plan = Arc::new(registered(DerivedKind::ClosureAdjacency).prepare(
            &ViewRequest::ClosureAdjacency {
                source,
                destination,
            },
            &schema,
        )?);
        let owner = Rc::new(RelationOwner::new(
            true,
            crate::outbound::derived::factory::test_port(),
        ));
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
        let crate::domain::derived_views::types::DerivedView::PartitionOrder(order) =
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
        drop(self.current_lease.take());
        self.current_lease = Some(self.owner.derived.scoped_build(
            crate::application::file_dag::route_counts::BuildRoute::ClosurePartition,
            &self.closure_plan,
            DerivedInputs::Partition {
                rows: self.input,
                positions: &order[begin..self.cursor],
                key: &self.key,
            },
            &mut self.current,
            &mut self.scratch,
        )?);
        Ok(true)
    }
    pub(in crate::application::file_dag) fn take_current(
        &mut self,
    ) -> Result<(ClosurePartition, Arc<ResourceLease>), OperatingError> {
        let lease = self.current_lease.take().ok_or(OperatingError::NotRun)?;
        let DerivedStorage::ClosureAdjacency(current) = std::mem::take(&mut self.current) else {
            return Err(OperatingError::NotRun);
        };
        Ok((current, lease))
    }
    pub(crate) fn current(&self) -> &ClosurePartition {
        let crate::domain::derived_views::types::DerivedView::ClosureAdjacency(current) =
            crate::domain::derived_views::collections::views::registration::registered(
                self.closure_plan.kind,
            )
            .view(&self.closure_plan, &self.current)
            .expect("validated current partition")
        else {
            unreachable!("prepared closure adjacency")
        };
        current
    }

    /// All live Value payload cells owned by this cursor, not only adjacency rows.
    pub(crate) fn working_cells(&self) -> usize {
        self.current().table.cells.len()
            + self.current().key.len()
            + self.current().sources.len()
            + self.key.len()
            + self.scratch.row.len()
    }

    /// All reserved Value slots, including the reusable key and source buffers.
    pub(crate) fn working_capacity(&self) -> usize {
        self.current().table.cells.capacity()
            + self.current().key.capacity()
            + self.current().sources.capacity()
            + self.key.capacity()
            + self.scratch.row.capacity()
    }

    /// Retained capacity accounted across the entire cursor: row IDs, key
    /// metadata, current adjacency/index, and Value payload. This is capacity
    /// accounting, not allocator headers or process RSS; source rows are borrowed.
    pub(crate) fn ordering_bytes(&self) -> usize {
        fn vector<T>(capacity: usize) -> usize {
            capacity.saturating_mul(size_of::<T>())
        }
        let index = &self.current().index;
        vector::<RowIndex>(self.ordering_capacity)
            .saturating_add(vector::<Value>(self.working_capacity()))
            .saturating_add(vector::<CellKind>(self.current().table.kinds.capacity()))
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

use ryo_values::domain::rows::types::RowIndex;
