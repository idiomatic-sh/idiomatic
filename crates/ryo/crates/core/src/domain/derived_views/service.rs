//! Static request interpretation and immutable access; no acquisition or retained policy.
use super::types::*;
use crate::domain::{
    operating::errors::OperatingError, operation_schema::types::InputSchema,
    table_model::service::check_columns,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(crate) fn kind(request: &ViewRequest) -> DerivedKind {
    match request {
        ViewRequest::Equality { .. } => DerivedKind::Equality,
        ViewRequest::PredicateMatches(_) => DerivedKind::PredicateMatches,
        ViewRequest::CandidatePostings { .. } => DerivedKind::CandidatePostings,
        ViewRequest::DistinctRepresentatives { .. } => DerivedKind::DistinctRepresentatives,
        ViewRequest::Adjacency { .. } => DerivedKind::Adjacency,
        ViewRequest::Membership { .. } => DerivedKind::Membership,
        ViewRequest::PartitionOrder { .. } => DerivedKind::PartitionOrder,
        ViewRequest::ClosureAdjacency { .. } => DerivedKind::ClosureAdjacency,
        ViewRequest::CycleAdjacency { .. } => DerivedKind::CycleAdjacency,
        ViewRequest::OrderedKeys { .. } => DerivedKind::OrderedKeys,
        ViewRequest::OrderedDfs(_) => DerivedKind::OrderedDfs,
        ViewRequest::NativeLookup { .. } => DerivedKind::NativeLookup,
    }
}
fn schema<'a>(
    schemas: &'a [InputSchema<'_>],
    at: usize,
) -> Result<&'a [ryo_language::domain::relations::types::CellKind], OperatingError> {
    schemas
        .get(at)
        .map(|schema| schema.kinds)
        .ok_or(OperatingError::MissingInput { position: at + 1 })
}
fn check(
    schemas: &[InputSchema<'_>],
    at: usize,
    columns: &[ColumnIndex],
) -> Result<(), OperatingError> {
    check_columns(schema(schemas, at)?, columns).map_err(Into::into)
}
/// Metadata failures are stored, not raised before the caller's existing lazy demand gate.
pub(crate) fn prepare(
    expected: DerivedKind,
    request: &ViewRequest,
    schemas: &[InputSchema<'_>],
) -> Result<DerivedContract, OperatingError> {
    if kind(request) != expected {
        return Err(OperatingError::WrongOperation);
    }
    let metadata = (|| match request {
        ViewRequest::Equality { columns } | ViewRequest::DistinctRepresentatives { columns } => {
            check(schemas, 0, columns)
        }
        ViewRequest::PredicateMatches(_) => {
            schema(schemas, 0)?;
            Ok(())
        }
        ViewRequest::CandidatePostings { column, .. } | ViewRequest::Membership { column } => {
            check(schemas, 0, &[*column])
        }
        ViewRequest::Adjacency { endpoints, keys } => {
            check(schemas, 0, endpoints)?;
            check(schemas, 0, keys)
        }
        ViewRequest::PartitionOrder { keys } => check(schemas, 0, keys),
        ViewRequest::ClosureAdjacency {
            source,
            destination,
        }
        | ViewRequest::CycleAdjacency {
            source,
            destination,
        } => check(schemas, 0, &[*source, *destination]),
        ViewRequest::OrderedKeys {
            left_on,
            right_on,
            left_order,
            right_order,
        } => {
            check(schemas, 0, left_on)?;
            check(schemas, 1, right_on)?;
            check(schemas, 0, &[*left_order])?;
            check(schemas, 1, &[*right_order])
        }
        ViewRequest::OrderedDfs(program) => {
            check(schemas, 0, &[program.root])?;
            check(schemas, 0, &program.root_keys)?;
            check(
                schemas,
                1,
                &[
                    program.source,
                    program.destination,
                    program.order,
                    program.edge,
                ],
            )?;
            check(schemas, 1, &program.edge_keys)
        }
        ViewRequest::NativeLookup {
            left_on,
            right_on,
            view,
        } => {
            check(schemas, 0, left_on)?;
            check(schemas, 1, right_on)?;
            check(schemas, 1, view)
        }
    })();
    Ok(DerivedContract {
        needed: Default::default(),
        kind: expected,
        request: request.clone(),
        schemas: schemas.iter().map(|schema| schema.kinds.to_vec()).collect(),
        metadata,
    })
}
pub(crate) fn validate(
    expected: DerivedKind,
    contract: &DerivedContract,
) -> Result<(), OperatingError> {
    if contract.kind != expected || kind(&contract.request) != expected {
        return Err(OperatingError::WrongOperation);
    }
    contract.metadata.clone()
}
/// Access is a finite match over built storage; it cannot touch a source population.
pub(crate) fn view<'a>(
    expected: DerivedKind,
    contract: &DerivedContract,
    storage: &'a DerivedStorage,
) -> Result<DerivedView<'a>, OperatingError> {
    validate(expected, contract)?;
    match (expected, storage) {
        (DerivedKind::Equality, DerivedStorage::Equality(index)) => {
            Ok(DerivedView::Equality(index))
        }
        (DerivedKind::Equality, DerivedStorage::EqualityShared(index)) => {
            Ok(DerivedView::Equality(index))
        }
        (
            DerivedKind::PredicateMatches | DerivedKind::DistinctRepresentatives,
            DerivedStorage::RowIds { kind, rows },
        ) if *kind == expected => Ok(DerivedView::RowIds(rows)),
        (DerivedKind::CandidatePostings, DerivedStorage::Postings(rows)) => {
            Ok(DerivedView::Postings(rows))
        }
        (DerivedKind::Adjacency, DerivedStorage::Adjacency(graph)) => {
            Ok(DerivedView::Adjacency(graph))
        }
        (DerivedKind::Membership, DerivedStorage::Membership(members)) => {
            Ok(DerivedView::Membership(members))
        }
        (DerivedKind::PartitionOrder, DerivedStorage::PartitionOrder(order)) => {
            Ok(DerivedView::PartitionOrder(order))
        }
        (DerivedKind::ClosureAdjacency, DerivedStorage::ClosureAdjacency(part)) => {
            Ok(DerivedView::ClosureAdjacency(part))
        }
        (DerivedKind::CycleAdjacency, DerivedStorage::CycleAdjacency(graph)) => {
            Ok(DerivedView::CycleAdjacency(graph))
        }
        (DerivedKind::OrderedKeys, DerivedStorage::OrderedKeys(keys)) => {
            Ok(DerivedView::OrderedKeys(keys))
        }
        (DerivedKind::OrderedDfs, DerivedStorage::OrderedDfs(graph)) => {
            Ok(DerivedView::OrderedDfs(graph))
        }
        (DerivedKind::NativeLookup, DerivedStorage::NativeLookup(finding)) => {
            Ok(DerivedView::NativeLookup(*finding))
        }
        (_, DerivedStorage::Unbuilt) => Err(OperatingError::NotRun),
        _ => Err(OperatingError::WrongOperation),
    }
}

/// Constant prepared-slot lookup; runtime hits never scan or clone request metadata.
pub(crate) fn slot(kind: DerivedKind) -> usize {
    match kind {
        DerivedKind::Equality => 0,
        DerivedKind::PredicateMatches => 1,
        DerivedKind::CandidatePostings => 2,
        DerivedKind::DistinctRepresentatives => 3,
        DerivedKind::Adjacency => 4,
        DerivedKind::Membership => 5,
        DerivedKind::PartitionOrder => 6,
        DerivedKind::ClosureAdjacency => 7,
        DerivedKind::CycleAdjacency => 8,
        DerivedKind::OrderedKeys => 9,
        DerivedKind::OrderedDfs => 10,
        DerivedKind::NativeLookup => 11,
    }
}

impl std::ops::Deref for super::types::PreparedViewGroup {
    type Target = super::types::PreparedViewUse;
    fn deref(&self) -> &Self::Target {
        &self.primary
    }
}
