//! Exhaustive semantic dispatcher. No caching, publication or admission decisions.
use crate::domain::contract::types::derived_payload::value::{DerivedPayload, PayloadData};
use crate::domain::contract::types::prepared_derived::value::PreparedDerived;
use crate::domain::contract::types::{
    BuildInputs, BuildOutcome, BuildWorkspace, DerivedView,
};
use crate::domain::dfs::types::DfsColumns;
use crate::domain::fault::types::DerivedFault;
use crate::domain::membership::types::{DerivedMembership, MembershipView};
use crate::domain::native::types::{NativeFinding, NativeForm};
use crate::domain::positions::types::{DenseMarks, RowSlots};
use crate::domain::request::types::{DerivedKind, DerivedRequest};
use std::sync::Arc;

/// Builds the payload a prepared request asks for from inputs of its kind, reusing a recycled payload's buffers.
///
/// # Errors
/// The fault of the request, the inputs or the build.
pub fn build(
    prepared: &PreparedDerived,
    inputs: BuildInputs<'_>,
    room: &mut BuildWorkspace,
    recycled: Option<DerivedPayload>,
) -> Result<BuildOutcome, DerivedFault> {
    prepared.validate()?;
    let recycled = recycled.map(|payload| payload.data);
    let data = match (&prepared.request, inputs) {
        (DerivedRequest::Equality { columns }, BuildInputs::Rows(rows)) => {
            let previous = match recycled {
                Some(PayloadData::Equality(value)) => match Arc::try_unwrap(value) {
                    Ok(owned) => Some(owned),
                    Err(_) => None,
                },
                _ => None,
            };
            PayloadData::Equality(Arc::new(crate::domain::equality::service::build(
                rows, columns, previous,
            )?))
        }
        (DerivedRequest::PredicateMatches { .. }, BuildInputs::Predicate { rows, candidates }) => {
            PayloadData::PredicateMatches(crate::domain::predicate::service::matches(
                rows, candidates,
            )?)
        }
        (
            DerivedRequest::CandidatePostings { column, literals },
            BuildInputs::Postings {
                rows,
                symbols,
                source,
                active,
            },
        ) => PayloadData::CandidatePostings(crate::domain::predicate::service::postings(
            rows, *column, literals, active, symbols, source,
        )?),
        (
            DerivedRequest::DistinctRepresentatives { columns },
            BuildInputs::Distinct { rows, positions },
        ) => PayloadData::DistinctRepresentatives(crate::domain::predicate::service::distinct(
            rows, columns, positions,
        )?),
        (DerivedRequest::Adjacency { endpoints, keys }, BuildInputs::Rows(rows)) => {
            let Some(graph) =
                crate::domain::adjacency::service::build_borrowed(rows, *endpoints, keys)?
            else {
                return Ok(BuildOutcome::Declined);
            };
            PayloadData::Adjacency(Arc::new(graph))
        }
        (DerivedRequest::Membership { column }, BuildInputs::Membership { rows, space }) => {
            PayloadData::Membership(crate::domain::membership::service::build(
                rows, *column, space,
            )?)
        }
        (DerivedRequest::PartitionOrder { keys }, BuildInputs::Rows(rows)) => {
            PayloadData::PartitionOrder(crate::domain::partition::service::ordered_positions(
                rows, keys,
            )?)
        }
        (
            DerivedRequest::ClosureAdjacency {
                source,
                destination,
            },
            BuildInputs::Partition {
                rows,
                positions,
                key,
            },
        ) => {
            let previous = match recycled {
                Some(PayloadData::ClosureAdjacency(value)) => match Arc::try_unwrap(value) {
                    Ok(owned) => Some(owned),
                    Err(_) => None,
                },
                _ => None,
            };
            PayloadData::ClosureAdjacency(Arc::new(crate::domain::closure_adjacency::service::build(
                rows,
                positions,
                key,
                *source,
                *destination,
                previous,
                &mut room.row,
            )?))
        }
        (
            DerivedRequest::CycleAdjacency {
                source,
                destination,
            },
            BuildInputs::Rows(rows),
        ) => PayloadData::CycleAdjacency(Arc::new(crate::domain::cycle_adjacency::service::build(
            rows,
            *source,
            *destination,
            &mut room.row,
        )?)),
        (
            DerivedRequest::OrderedKeys {
                left_on,
                right_on,
                left_order,
                right_order,
            },
            BuildInputs::Ordered { left, right, texts },
        ) => PayloadData::OrderedKeys(Arc::new(crate::domain::ordered::service::build(
            left,
            right,
            left_on,
            right_on,
            *left_order,
            *right_order,
            texts,
            &mut room.row,
        )?)),
        (
            DerivedRequest::OrderedDfs {
                root,
                root_keys,
                source,
                destination,
                order,
                edge,
                edge_keys,
            },
            BuildInputs::Dfs {
                roots,
                edges,
                texts,
            },
        ) => PayloadData::OrderedDfs(Arc::new(crate::domain::dfs::service::build(
            DfsColumns {
                root: *root,
                root_keys,
                source: *source,
                destination: *destination,
                order: *order,
                edge: *edge,
                edge_keys,
            },
            roots,
            edges,
            texts,
            &mut room.row,
        )?)),
        (DerivedRequest::NativeLookup { .. }, BuildInputs::Native(form)) => {
            let Some(finding) = native_finding(prepared, form)? else {
                return Ok(BuildOutcome::Declined);
            };
            PayloadData::NativeLookup(finding)
        }
        (DerivedRequest::RawKeyOrder { stride }, BuildInputs::RawWords(keys)) => {
            let previous = match recycled {
                Some(PayloadData::RawKeyOrder(value)) => match Arc::try_unwrap(value) {
                    Ok(owned) => Some(owned),
                    Err(_) => None,
                },
                _ => None,
            };
            PayloadData::RawKeyOrder(Arc::new(crate::domain::raw_order::service::build(
                keys, *stride, previous,
            )))
        }
        _ => return Err(DerivedFault::WrongOperation),
    };
    Ok(BuildOutcome::Built(DerivedPayload { data }))
}

/// Borrows the view of a built payload of the request's kind.
///
/// # Errors
/// The stored metadata fault, or a payload of another kind.
pub fn view<'a>(
    prepared: &PreparedDerived,
    payload: &'a DerivedPayload,
) -> Result<DerivedView<'a>, DerivedFault> {
    prepared.validate()?;
    match (
        crate::domain::preparation::service::kind(&prepared.request),
        &payload.data,
    ) {
        (DerivedKind::Equality, PayloadData::Equality(data)) => Ok(DerivedView::Equality(data)),
        (DerivedKind::PredicateMatches, PayloadData::PredicateMatches(data))
        | (DerivedKind::DistinctRepresentatives, PayloadData::DistinctRepresentatives(data)) => {
            Ok(DerivedView::RowIds(RowSlots(data.0.as_slice())))
        }
        (DerivedKind::CandidatePostings, PayloadData::CandidatePostings(data)) => {
            Ok(DerivedView::Postings(data))
        }
        (DerivedKind::Adjacency, PayloadData::Adjacency(data)) => Ok(DerivedView::Adjacency(data)),
        (DerivedKind::Membership, PayloadData::Membership(data)) => {
            Ok(DerivedView::Membership(match data {
                DerivedMembership::Values(values) => MembershipView::Values(values),
                DerivedMembership::Dense(values) => MembershipView::Dense(DenseMarks(values.0.as_slice())),
            }))
        }
        (DerivedKind::PartitionOrder, PayloadData::PartitionOrder(data)) => {
            Ok(DerivedView::PartitionOrder(data))
        }
        (DerivedKind::ClosureAdjacency, PayloadData::ClosureAdjacency(data)) => {
            Ok(DerivedView::ClosureAdjacency(data))
        }
        (DerivedKind::CycleAdjacency, PayloadData::CycleAdjacency(data)) => {
            Ok(DerivedView::CycleAdjacency(data))
        }
        (DerivedKind::OrderedKeys, PayloadData::OrderedKeys(data)) => {
            Ok(DerivedView::OrderedKeys(data))
        }
        (DerivedKind::OrderedDfs, PayloadData::OrderedDfs(data)) => {
            Ok(DerivedView::OrderedDfs(data))
        }
        (DerivedKind::NativeLookup, PayloadData::NativeLookup(data)) => {
            Ok(DerivedView::NativeLookup(data))
        }
        (DerivedKind::RawKeyOrder, PayloadData::RawKeyOrder(data)) => {
            Ok(DerivedView::RawKeyOrder(data))
        }
        _ => Err(DerivedFault::WrongOperation),
    }
}

/// Interprets optional native support without building a lookup payload.
///
/// # Errors
/// The stored metadata fault, or a request that is not a native lookup.
pub fn native_finding(
    prepared: &PreparedDerived,
    form: NativeForm,
) -> Result<Option<NativeFinding>, DerivedFault> {
    prepared.validate()?;
    let DerivedRequest::NativeLookup {
        left_on,
        right_on,
        view,
    } = prepared.request()
    else {
        return Err(DerivedFault::WrongOperation);
    };
    Ok(crate::domain::native::service::finding(
        form, left_on, right_on, view,
    ))
}
