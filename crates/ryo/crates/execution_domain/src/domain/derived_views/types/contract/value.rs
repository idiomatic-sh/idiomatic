//! The sealed derived contract: only `issue` makes one, so its identity hash and kind cannot be forged.

use crate::domain::derived_views::types::{
    IdentityHash, InputsUsed, ViewRequest,
};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_relations::domain::relations::types::CellKind;
use std::sync::Arc;

/// The kind a request asks for.
pub fn kind_of(request: &ViewRequest) -> DerivedKind {
    match request {
        ViewRequest::Equality { .. } => DerivedKind::Equality,
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

/// One prepared derived view: its semantics, kind, request, input schemas and metadata, under one identity.
#[derive(Clone, Debug)]
pub struct DerivedContract {
    /// Prepared once; equality still checks the complete semantic descriptor on collisions.
    /// Private: a struct literal or update outside this module cannot forge a contract.
    identity_hash: IdentityHash,
    /// The derived crate's prepared semantics of the view.
    pub semantic: Arc<PreparedDerived>,
    /// The kind of the view.
    pub kind: DerivedKind,
    /// The view the operation requests.
    pub request: ViewRequest,
    /// The cell kinds of each input, in input order.
    pub schemas: Vec<Vec<CellKind>>,
    /// Whether the view's metadata was admitted, or why not.
    pub metadata: Result<(), crate::domain::operating::errors::OperatingError>,
}

impl DerivedContract {
    /// The only constructor: kind and metadata follow from the request and the validated semantic
    /// descriptor, so no caller can state a contract whose parts disagree.
    pub fn issue(
        identity_hash: IdentityHash,
        semantic: Arc<PreparedDerived>,
        request: ViewRequest,
        schemas: Vec<Vec<CellKind>>,
    ) -> Self {
        let metadata = semantic
            .validate()
            .map_err(crate::domain::operating::service::derived_fault);
        Self {
            identity_hash,
            kind: kind_of(&request),
            semantic,
            request,
            schemas,
            metadata,
        }
    }

    /// The identity the contract was prepared under.
    pub fn identity_hash(&self) -> IdentityHash {
        self.identity_hash
    }
}

/// Request-owned prepared use; population slots are immutable dependency metadata, not a scheduler.
#[derive(Clone, Debug)]
pub struct PreparedViewUse {
    /// The prepared view this use reads.
    pub plan: Arc<DerivedContract>,
    /// The inputs the view is built over.
    pub inputs: [InputNumber; 2],
    /// The relation each input reads, when it reads one directly.
    pub relations: [Option<ryo_relations::domain::relations::types::Relation>; 2],
    /// How many of the inputs the view uses.
    pub len: InputsUsed,
}
/// The views one derived slot of an operation reads: its primary view and an optional second one.
#[derive(Clone, Debug)]
pub struct PreparedViewGroup {
    /// The view the slot reads first.
    pub primary: Arc<PreparedViewUse>,
    /// The second view the slot reads, when it reads two.
    pub secondary: Option<Arc<PreparedViewUse>>,
}
/// The derived views an operation reads, one slot per derived kind.
#[derive(Clone, Debug, Default)]
pub struct DerivedSlots {
    /// The view group of each derived kind, when the operation reads one.
    pub slots: [Option<Arc<PreparedViewGroup>>; DerivedKind::ALL.len()],
}

