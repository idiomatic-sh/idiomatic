//! Preparation never reads rows, emits errors early or chooses retention.
use crate::domain::contract::types::prepared_derived::value::PreparedDerived;
use crate::domain::fault::types::{DerivedFault, InputSlot};
use crate::domain::request::types::{DerivedKind, DerivedRequest};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The kind of derived view a request asks for.
#[must_use]
pub fn kind(request: &DerivedRequest) -> DerivedKind {
    match request {
        DerivedRequest::Equality { .. } => DerivedKind::Equality,
        DerivedRequest::Adjacency { .. } => DerivedKind::Adjacency,
        DerivedRequest::Membership { .. } => DerivedKind::Membership,
        DerivedRequest::PartitionOrder { .. } => DerivedKind::PartitionOrder,
        DerivedRequest::ClosureAdjacency { .. } => DerivedKind::ClosureAdjacency,
        DerivedRequest::CycleAdjacency { .. } => DerivedKind::CycleAdjacency,
        DerivedRequest::OrderedKeys { .. } => DerivedKind::OrderedKeys,
        DerivedRequest::OrderedDfs { .. } => DerivedKind::OrderedDfs,
        DerivedRequest::NativeLookup { .. } => DerivedKind::NativeLookup,
    }
}
fn check(
    schemas: &[Vec<CellKind>],
    slot: InputSlot,
    columns: &[ColumnIndex],
) -> Result<(), DerivedFault> {
    let at = slot.0;
    let schema = schemas
        .get(at)
        .ok_or(DerivedFault::MissingInput { position: InputSlot(at + 1) })?;
    for column in columns {
        if usize::try_from(column.0).map_or(true, |index| index >= schema.len()) {
            return Err(DerivedFault::NoSuchColumn { column: *column });
        }
    }
    Ok(())
}
/// Checks a request against the schemas of its inputs and keeps the verdict.
#[must_use]
pub fn prepare(mut request: DerivedRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived {
    let metadata = (|| match &mut request {
        DerivedRequest::Equality { columns } => check(&schemas, InputSlot(0), columns),
        DerivedRequest::Membership { column } => check(&schemas, InputSlot(0), &[*column]),
        DerivedRequest::Adjacency { endpoints, keys } => {
            check(&schemas, InputSlot(0), endpoints)?;
            check(&schemas, InputSlot(0), keys)
        }
        DerivedRequest::PartitionOrder { keys } => check(&schemas, InputSlot(0), keys),
        DerivedRequest::ClosureAdjacency {
            source,
            destination,
        }
        | DerivedRequest::CycleAdjacency {
            source,
            destination,
        } => check(&schemas, InputSlot(0), &[*source, *destination]),
        DerivedRequest::OrderedKeys {
            left_on,
            right_on,
            left_order,
            right_order,
        } => {
            check(&schemas, InputSlot(0), left_on)?;
            check(&schemas, InputSlot(1), right_on)?;
            check(&schemas, InputSlot(0), &[*left_order])?;
            check(&schemas, InputSlot(1), &[*right_order])
        }
        DerivedRequest::OrderedDfs {
            root,
            root_keys,
            source,
            destination,
            order,
            edge,
            edge_keys,
        } => {
            check(&schemas, InputSlot(0), &[*root])?;
            check(&schemas, InputSlot(0), root_keys)?;
            check(&schemas, InputSlot(1), &[*source, *destination, *order, *edge])?;
            check(&schemas, InputSlot(1), edge_keys)
        }
        DerivedRequest::NativeLookup {
            left_on,
            right_on,
            view,
        } => {
            check(&schemas, InputSlot(0), left_on)?;
            check(&schemas, InputSlot(1), right_on)?;
            check(&schemas, InputSlot(1), view)
        }
    })();
    PreparedDerived {
        request,
        schemas,
        metadata,
    }
}
