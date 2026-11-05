//! Preparation never reads rows, emits errors early or chooses retention.
use super::contract::types::{DerivedFault, DerivedKind, PreparedDerived, ViewRequest};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(crate) fn kind(request: &ViewRequest) -> DerivedKind {
    match request {
        ViewRequest::Equality { .. } => DerivedKind::Equality,
        ViewRequest::PredicateMatches { .. } => DerivedKind::PredicateMatches,
        ViewRequest::CandidatePostings { .. } => DerivedKind::CandidatePostings,
        ViewRequest::DistinctRepresentatives { .. } => DerivedKind::DistinctRepresentatives,
        ViewRequest::Adjacency { .. } => DerivedKind::Adjacency,
        ViewRequest::Membership { .. } => DerivedKind::Membership,
        ViewRequest::PartitionOrder { .. } => DerivedKind::PartitionOrder,
        ViewRequest::ClosureAdjacency { .. } => DerivedKind::ClosureAdjacency,
        ViewRequest::CycleAdjacency { .. } => DerivedKind::CycleAdjacency,
        ViewRequest::OrderedKeys { .. } => DerivedKind::OrderedKeys,
        ViewRequest::OrderedDfs { .. } => DerivedKind::OrderedDfs,
        ViewRequest::NativeLookup { .. } => DerivedKind::NativeLookup,
        ViewRequest::RawKeyOrder { .. } => DerivedKind::RawKeyOrder,
    }
}
fn check(
    schemas: &[Vec<CellKind>],
    at: usize,
    columns: &[ColumnIndex],
) -> Result<(), DerivedFault> {
    let schema = schemas
        .get(at)
        .ok_or(DerivedFault::MissingInput { position: at + 1 })?;
    for column in columns {
        if usize::try_from(column.0).map_or(true, |index| index >= schema.len()) {
            return Err(DerivedFault::NoSuchColumn { column: *column });
        }
    }
    Ok(())
}
pub(crate) fn prepare(mut request: ViewRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived {
    let metadata = (|| match &mut request {
        ViewRequest::Equality { columns } | ViewRequest::DistinctRepresentatives { columns } => {
            check(&schemas, 0, columns)
        }
        ViewRequest::PredicateMatches { .. } => check(&schemas, 0, &[]),
        ViewRequest::CandidatePostings { column, literals } => {
            check(&schemas, 0, &[*column])?;
            literals.sort_by(|left, right| left.0.cmp(&right.0));
            literals.dedup_by(|left, right| left.0 == right.0);
            if schemas
                .first()
                .and_then(|schema| schema.get(column.0 as usize))
                != Some(&CellKind::Symbol)
                || literals.iter().any(|text| text.0.is_empty())
            {
                return Err(DerivedFault::ShapeMismatch);
            }
            Ok(())
        }
        ViewRequest::Membership { column } => check(&schemas, 0, &[*column]),
        ViewRequest::Adjacency { endpoints, keys } => {
            check(&schemas, 0, endpoints)?;
            check(&schemas, 0, keys)
        }
        ViewRequest::PartitionOrder { keys } => check(&schemas, 0, keys),
        ViewRequest::ClosureAdjacency {
            source,
            destination,
        }
        | ViewRequest::CycleAdjacency {
            source,
            destination,
        } => check(&schemas, 0, &[*source, *destination]),
        ViewRequest::OrderedKeys {
            left_on,
            right_on,
            left_order,
            right_order,
        } => {
            check(&schemas, 0, left_on)?;
            check(&schemas, 1, right_on)?;
            check(&schemas, 0, &[*left_order])?;
            check(&schemas, 1, &[*right_order])
        }
        ViewRequest::OrderedDfs {
            root,
            root_keys,
            source,
            destination,
            order,
            edge,
            edge_keys,
        } => {
            check(&schemas, 0, &[*root])?;
            check(&schemas, 0, root_keys)?;
            check(&schemas, 1, &[*source, *destination, *order, *edge])?;
            check(&schemas, 1, edge_keys)
        }
        ViewRequest::NativeLookup {
            left_on,
            right_on,
            view,
        } => {
            check(&schemas, 0, left_on)?;
            check(&schemas, 1, right_on)?;
            check(&schemas, 1, view)
        }
        ViewRequest::RawKeyOrder { .. } => Ok(()),
    })();
    PreparedDerived {
        request,
        schemas,
        metadata,
    }
}
