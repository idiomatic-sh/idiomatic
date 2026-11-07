//! The sealed derived contract: only `issue` makes one, so its identity hash and kind cannot be forged.

use crate::domain::derived_views::types::{
    IdentityHash, InputsUsed, SelectorAddress, ViewGroup, ViewGroupCount, ViewRequest,
};
use crate::domain::stored_selection::types::Selector;
use ryo_values::domain::plan_parts::types::{ColumnIndex, InputNumber};
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::request::types::DerivedKind;
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

/// Request-owned prepared use; population slots are immutable dependency metadata, not a scheduler.
#[derive(Clone, Debug)]
pub(crate) struct PreparedViewUse {
    pub plan: Arc<DerivedContract>,
    pub inputs: [InputNumber; 2],
    pub relations: [Option<ryo_relations::domain::relations::types::Relation>; 2],
    pub len: InputsUsed,
}
#[derive(Clone, Debug)]
pub(crate) struct PreparedViewGroup {
    pub primary: Arc<PreparedViewUse>,
    pub secondary: Option<Arc<PreparedViewUse>>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct DerivedSlots {
    pub slots: [Option<Arc<PreparedViewGroup>>; DerivedKind::ALL.len()],
}

/// Prepared construction contracts indexed by exact request-owned selector identity.
#[derive(Debug, Default)]
pub(crate) struct PreparedSelectionViews {
    /// Pins every selector address for the complete request lifetime.
    pub selectors: Vec<Arc<Selector>>,
    /// O(1) capability lookup; no metadata construction occurs while selecting.
    pub by_selector: std::collections::HashMap<SelectorAddress, Arc<PreparedSelectorViews>>,
    pub groups: ViewGroupCount,
}
#[derive(Debug)]
pub(crate) struct PreparedSelectorViews {
    pub group: ViewGroup,
    pub predicate: Arc<DerivedContract>,
    pub postings: Arc<DerivedContract>,
    pub distinct: Arc<DerivedContract>,
    /// Every statically possible equality discriminator, chosen without runtime preparation.
    pub equalities: std::collections::HashMap<ColumnIndex, Arc<DerivedContract>>,
}
