//! One prepared shared ordering and one reusable partition-local adjacency.
use crate::domain::{
    derived_views::types::{
        DerivedContract, DerivedInputs, DerivedScratch, DerivedStorage, ViewRequest,
    },
    operating::errors::OperatingError,
    operator::contract::{graph::ClosurePartition, views::BorrowedRows},
    table_cells::types::RowIndex,
    table_model::types::{ChainLink, HashedRow, KeyCellsHash},
};
use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

/// Ordering metadata can be shared across consumers; only the current partition
/// owns values/adjacency. Advancing releases the previous partition's logical contents.
pub(crate) struct PartitionedClosure<'f> {
    input: BorrowedRows<'f>,
    partition_plan: Arc<DerivedContract>,
    closure_plan: Arc<DerivedContract>,
    ordered: Arc<DerivedStorage>,
    cursor: usize,
    key: Vec<Value>,
    current: DerivedStorage,
    scratch: DerivedScratch,
}
impl<'f> PartitionedClosure<'f> {
    pub(crate) fn from_prepared(
        input: BorrowedRows<'f>,
        partition_plan: Arc<DerivedContract>,
        closure_plan: Arc<DerivedContract>,
        ordered: Arc<DerivedStorage>,
    ) -> Result<Self, OperatingError> {
        crate::domain::derived_views::service::validate(
            crate::domain::derived_views::types::DerivedKind::PartitionOrder,
            &partition_plan,
        )?;
        crate::domain::derived_views::service::validate(
            crate::domain::derived_views::types::DerivedKind::ClosureAdjacency,
            &closure_plan,
        )?;
        if !matches!(ordered.as_ref(), DerivedStorage::PartitionOrder(_)) {
            return Err(OperatingError::WrongOperation);
        }
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
            table: crate::domain::table_model::types::Table {
                kinds: vec![schema[source.0 as usize], schema[destination.0 as usize]],
                ..Default::default()
            },
        });
        Ok(Self {
            input,
            partition_plan,
            closure_plan,
            ordered,
            cursor: 0,
            key: Vec::new(),
            current,
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
        use crate::domain::derived_views::{
            collections::views::registration::registered, types::DerivedKind,
        };
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
        let mut ordered = DerivedStorage::Unbuilt;
        super::derived::owner::build_scoped(
            &partition_plan,
            DerivedInputs::Rows(&[input]),
            &mut ordered,
            &mut DerivedScratch::default(),
        )?;
        Self::from_prepared(input, partition_plan, closure_plan, Arc::new(ordered))
    }
    fn order(&self) -> &[RowIndex] {
        let crate::domain::derived_views::types::DerivedView::PartitionOrder(order) =
            crate::domain::derived_views::collections::views::registration::registered(
                self.partition_plan.kind,
            )
            .view(&self.partition_plan, &self.ordered)
            .expect("validated partition ordering")
        else {
            unreachable!("prepared partition order")
        };
        order
    }
    pub(crate) fn advance(&mut self) -> Result<bool, OperatingError> {
        let Some(&first) = self.order().get(self.cursor) else {
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
        while let Some(&row) = self.order().get(self.cursor) {
            if !keys
                .iter()
                .zip(&self.key)
                .all(|(column, value)| self.input.cell(row, *column) == Some(*value))
            {
                break;
            }
            self.cursor += 1;
        }
        let crate::domain::derived_views::types::DerivedView::PartitionOrder(order) =
            crate::domain::derived_views::collections::views::registration::registered(
                self.partition_plan.kind,
            )
            .view(&self.partition_plan, &self.ordered)?
        else {
            return Err(OperatingError::WrongOperation);
        };
        super::derived::owner::build_scoped(
            &self.closure_plan,
            DerivedInputs::Partition {
                rows: self.input,
                positions: &order[begin..self.cursor],
                key: &self.key,
            },
            &mut self.current,
            &mut self.scratch,
        )?;
        Ok(true)
    }
    pub(crate) fn take_current(&mut self) -> Result<ClosurePartition, OperatingError> {
        let DerivedStorage::ClosureAdjacency(current) = std::mem::take(&mut self.current) else {
            return Err(OperatingError::NotRun);
        };
        Ok(current)
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
        vector::<RowIndex>(match self.ordered.as_ref() {
            DerivedStorage::PartitionOrder(order) => order.capacity(),
            _ => 0,
        })
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
