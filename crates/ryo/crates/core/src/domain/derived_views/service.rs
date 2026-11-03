//! Static request interpretation and immutable access; no acquisition or retained policy.
use super::types::*;
use crate::domain::operating::errors::OperatingError;

pub(crate) fn kind(request: &ViewRequest) -> DerivedKind {
    match request {
        ViewRequest::RawKeyOrder { .. } => DerivedKind::RawKeyOrder,
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
pub(crate) fn validate(
    expected: DerivedKind,
    contract: &DerivedContract,
) -> Result<(), OperatingError> {
    if contract.kind != expected || kind(&contract.request) != expected {
        return Err(OperatingError::WrongOperation);
    }
    contract.metadata.clone()
}
/// Constant prepared-slot lookup; runtime hits never scan or clone request metadata.
pub(crate) fn slot(kind: DerivedKind) -> usize {
    kind.ordinal()
}

use ryo_derived::domain::contract::types::DerivedKind;
