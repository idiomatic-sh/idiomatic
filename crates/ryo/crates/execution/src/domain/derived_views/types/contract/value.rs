//! The sealed derived contract: only `issue` makes one, so its identity hash and kind cannot be forged.

use crate::domain::derived_views::types::{IdentityHash, ViewRequest};
use ryo_derived::domain::contract::types::{DerivedKind, PreparedDerived};
use ryo_relations::domain::relations::types::CellKind;
use std::sync::Arc;

/// The kind a request asks for.
pub(crate) fn kind_of(request: &ViewRequest) -> DerivedKind {
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

#[derive(Clone, Debug)]
pub(crate) struct DerivedContract {
    /// Prepared once; equality still checks the complete semantic descriptor on collisions.
    /// Private: a struct literal or update outside this module cannot forge a contract.
    identity_hash: IdentityHash,
    pub semantic: Arc<PreparedDerived>,
    pub kind: DerivedKind,
    pub request: ViewRequest,
    pub schemas: Vec<Vec<CellKind>>,
    pub metadata: Result<(), crate::domain::operating::errors::OperatingError>,
    pub needed: crate::domain::table_cells::types::ColumnPlaces,
}

impl DerivedContract {
    /// The only constructor: kind and metadata follow from the request and the validated semantic
    /// descriptor, so no caller can state a contract whose parts disagree.
    pub(crate) fn issue(
        identity_hash: IdentityHash,
        semantic: Arc<PreparedDerived>,
        request: ViewRequest,
        schemas: Vec<Vec<CellKind>>,
        needed: crate::domain::table_cells::types::ColumnPlaces,
    ) -> Self {
        let metadata = semantic
            .validate()
            .map_err(crate::domain::operator::contract::derived::portable_fault);
        Self {
            identity_hash,
            kind: kind_of(&request),
            semantic,
            request,
            schemas,
            metadata,
            needed,
        }
    }

    pub(crate) fn identity_hash(&self) -> IdentityHash {
        self.identity_hash
    }
}
