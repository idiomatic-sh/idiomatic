//! Exhaustive semantic dispatcher. No caching, publication or admission decisions.
use super::contract::types::*;
use std::sync::Arc;
pub(crate) fn build(
    prepared: &PreparedDerived,
    inputs: DerivedInputs<'_>,
    room: &mut BuildWorkspace,
    recycled: Option<DerivedPayload>,
) -> Result<BuildOutcome, DerivedFault> {
    prepared.validate()?;
    let recycled = recycled.map(|payload| payload.data);
    let data = match (&prepared.request, inputs) {
        (ViewRequest::Equality { columns }, DerivedInputs::Rows(rows)) => {
            let previous = match recycled {
                Some(PayloadData::Equality(value)) => Arc::try_unwrap(value).ok(),
                _ => None,
            };
            PayloadData::Equality(Arc::new(super::equality::build(rows, columns, previous)?))
        }
        (ViewRequest::PredicateMatches { .. }, DerivedInputs::Predicate { rows, candidates }) => {
            PayloadData::PredicateMatches(super::predicate::matches(rows, candidates)?)
        }
        (
            ViewRequest::CandidatePostings { column, literals },
            DerivedInputs::Postings {
                rows,
                symbols,
                source,
                active,
            },
        ) => PayloadData::CandidatePostings(super::predicate::postings(
            rows, *column, literals, active, symbols, source,
        )?),
        (
            ViewRequest::DistinctRepresentatives { columns },
            DerivedInputs::Distinct { rows, positions },
        ) => PayloadData::DistinctRepresentatives(super::predicate::distinct(
            rows, columns, positions,
        )?),
        (ViewRequest::Adjacency { endpoints, keys }, DerivedInputs::Rows(rows)) => {
            let Some(graph) = super::adjacency::build_borrowed(rows, *endpoints, keys)? else {
                return Ok(BuildOutcome::Declined);
            };
            PayloadData::Adjacency(Arc::new(graph))
        }
        (ViewRequest::Membership { column }, DerivedInputs::Membership { rows, space }) => {
            PayloadData::Membership(super::membership::build(rows, *column, space)?)
        }
        (ViewRequest::PartitionOrder { keys }, DerivedInputs::Rows(rows)) => {
            PayloadData::PartitionOrder(super::partition::ordered_positions(rows, keys)?)
        }
        (
            ViewRequest::ClosureAdjacency {
                source,
                destination,
            },
            DerivedInputs::Partition {
                rows,
                positions,
                key,
            },
        ) => {
            let previous = match recycled {
                Some(PayloadData::ClosureAdjacency(value)) => Arc::try_unwrap(value).ok(),
                _ => None,
            };
            PayloadData::ClosureAdjacency(Arc::new(super::closure::build(
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
            ViewRequest::CycleAdjacency {
                source,
                destination,
            },
            DerivedInputs::Rows(rows),
        ) => PayloadData::CycleAdjacency(Arc::new(super::cycles::build(
            rows,
            *source,
            *destination,
            &mut room.row,
        )?)),
        (
            ViewRequest::OrderedKeys {
                left_on,
                right_on,
                left_order,
                right_order,
            },
            DerivedInputs::Ordered { left, right, texts },
        ) => PayloadData::OrderedKeys(Arc::new(super::ordered::build(
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
            ViewRequest::OrderedDfs { .. },
            DerivedInputs::Dfs {
                roots,
                edges,
                texts,
            },
        ) => PayloadData::OrderedDfs(Arc::new(super::dfs::build(
            &prepared.request,
            roots,
            edges,
            texts,
            &mut room.row,
        )?)),
        (
            ViewRequest::NativeLookup {
                left_on,
                right_on,
                view,
            },
            DerivedInputs::Native(form),
        ) => {
            let Some(finding) = super::native::finding(form, left_on, right_on, view) else {
                return Ok(BuildOutcome::Declined);
            };
            PayloadData::NativeLookup(finding)
        }
        (ViewRequest::RawKeyOrder { stride }, DerivedInputs::RawWords(keys)) => {
            let previous = match recycled {
                Some(PayloadData::RawKeyOrder(value)) => Arc::try_unwrap(value).ok(),
                _ => None,
            };
            PayloadData::RawKeyOrder(Arc::new(super::raw_order::build(keys, *stride, previous)))
        }
        _ => return Err(DerivedFault::WrongOperation),
    };
    Ok(BuildOutcome::Built(DerivedPayload { data }))
}
pub(crate) fn view<'a>(
    prepared: &PreparedDerived,
    payload: &'a DerivedPayload,
) -> Result<DerivedView<'a>, DerivedFault> {
    prepared.validate()?;
    match (super::preparation::kind(&prepared.request), &payload.data) {
        (DerivedKind::Equality, PayloadData::Equality(data)) => Ok(DerivedView::Equality(data)),
        (DerivedKind::PredicateMatches, PayloadData::PredicateMatches(data))
        | (DerivedKind::DistinctRepresentatives, PayloadData::DistinctRepresentatives(data)) => {
            Ok(DerivedView::RowIds(data.as_slice()))
        }
        (DerivedKind::CandidatePostings, PayloadData::CandidatePostings(data)) => {
            Ok(DerivedView::Postings(data))
        }
        (DerivedKind::Adjacency, PayloadData::Adjacency(data)) => Ok(DerivedView::Adjacency(data)),
        (DerivedKind::Membership, PayloadData::Membership(data)) => {
            Ok(DerivedView::Membership(match data {
                Membership::Values(values) => MembershipView::Values(values),
                Membership::Dense(values) => MembershipView::Dense(values.as_slice()),
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
