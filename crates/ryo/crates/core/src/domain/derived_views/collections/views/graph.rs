//! Pure graph-derived views. The application owns demand, storage lifetime and reuse.
use super::interface::DerivedViewsItem;
use crate::domain::{
    derived_views::{service, types::*},
    operating::errors::OperatingError,
    operation_schema::types::InputSchema,
    operator::contract::views::BorrowedRows,
};

mod closure;
mod cycles;
mod dfs;
mod ordered;
#[cfg(test)]
mod tests;

fn first<'a>(rows: &[BorrowedRows<'a>]) -> Result<BorrowedRows<'a>, OperatingError> {
    rows.first()
        .copied()
        .ok_or(OperatingError::MissingInput { position: 1 })
}

pub(crate) struct AdjacencyItem;
impl DerivedViewsItem for AdjacencyItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        service::prepare(DerivedKind::Adjacency, request, inputs)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        _scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::Adjacency, contract)?;
        let (ViewRequest::Adjacency { endpoints, keys }, DerivedInputs::Adjacency { rows, source }) =
            (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        let prepared_endpoints = match source.orientation {
            crate::domain::stored_graph::types::Orientation::Forward => *endpoints,
            crate::domain::stored_graph::types::Orientation::Reverse => {
                [endpoints[1], endpoints[0]]
            }
        };
        if source.endpoints != prepared_endpoints {
            return Err(OperatingError::ShapeMismatch);
        }
        let graph = crate::domain::stored_graph::service::build_borrowed(
            source.identity,
            rows,
            source.endpoints,
            keys,
        )?
        .ok_or(OperatingError::ShapeMismatch)?;
        *storage = DerivedStorage::Adjacency(std::sync::Arc::new(graph));
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::Adjacency, contract, storage)
    }
}

pub(crate) struct PartitionOrderItem;
impl DerivedViewsItem for PartitionOrderItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        service::prepare(DerivedKind::PartitionOrder, request, inputs)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        _scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::PartitionOrder, contract)?;
        let (ViewRequest::PartitionOrder { keys }, DerivedInputs::Rows(rows)) =
            (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        let order =
            crate::domain::operator::contract::partition::ordered_positions(first(rows)?, keys)?;
        *storage = DerivedStorage::PartitionOrder(order);
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::PartitionOrder, contract, storage)
    }
}

pub(crate) struct ClosureAdjacencyItem;
impl DerivedViewsItem for ClosureAdjacencyItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        service::prepare(DerivedKind::ClosureAdjacency, request, inputs)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::ClosureAdjacency, contract)?;
        let (
            ViewRequest::ClosureAdjacency {
                source,
                destination,
            },
            inputs,
        ) = (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        let (rows, positions, key) = match inputs {
            DerivedInputs::Rows(rows) => (first(rows)?, None, &[][..]),
            DerivedInputs::Partition {
                rows,
                positions,
                key,
            } => (rows, Some(positions), key),
            _ => return Err(OperatingError::WrongOperation),
        };
        let previous = match std::mem::take(storage) {
            DerivedStorage::ClosureAdjacency(part) => Some(part),
            _ => None,
        };
        let part = closure::prepare(
            rows,
            positions,
            key,
            *source,
            *destination,
            previous,
            &mut scratch.row,
        )?;
        *storage = DerivedStorage::ClosureAdjacency(part);
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::ClosureAdjacency, contract, storage)
    }
}

pub(crate) struct CycleAdjacencyItem;
impl DerivedViewsItem for CycleAdjacencyItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        service::prepare(DerivedKind::CycleAdjacency, request, inputs)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::CycleAdjacency, contract)?;
        let (
            ViewRequest::CycleAdjacency {
                source,
                destination,
            },
            DerivedInputs::Rows(rows),
        ) = (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        let graph = cycles::prepare(first(rows)?, *source, *destination, &mut scratch.row)?;
        *storage = DerivedStorage::CycleAdjacency(graph);
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::CycleAdjacency, contract, storage)
    }
}

pub(crate) struct OrderedKeysItem;
impl DerivedViewsItem for OrderedKeysItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        service::prepare(DerivedKind::OrderedKeys, request, inputs)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::OrderedKeys, contract)?;
        let (
            ViewRequest::OrderedKeys {
                left_on,
                right_on,
                left_order,
                right_order,
            },
            DerivedInputs::Text { rows, texts },
        ) = (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        let left = first(rows)?;
        let right = rows
            .get(1)
            .copied()
            .ok_or(OperatingError::MissingInput { position: 2 })?;
        let ordered = ordered::prepare(
            left,
            right,
            left_on,
            right_on,
            *left_order,
            *right_order,
            texts,
            &mut scratch.row,
        )?;
        *storage = DerivedStorage::OrderedKeys(ordered);
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::OrderedKeys, contract, storage)
    }
}

pub(crate) struct OrderedDfsItem;
impl DerivedViewsItem for OrderedDfsItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        service::prepare(DerivedKind::OrderedDfs, request, inputs)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::OrderedDfs, contract)?;
        let (
            ViewRequest::OrderedDfs(program),
            DerivedInputs::Dfs {
                roots,
                edges,
                symbols,
                source,
                paths,
                arenas,
            },
        ) = (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        let prepared = dfs::prepare(
            program,
            roots,
            edges,
            symbols,
            source,
            paths,
            arenas,
            &mut scratch.row,
        )?;
        *storage = DerivedStorage::OrderedDfs(prepared);
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::OrderedDfs, contract, storage)
    }
}

use ryo_derived::domain::contract::types::DerivedKind;
