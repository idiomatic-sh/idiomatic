//! The built payload of one derived kind: the value and its behaviour, owned together.
use crate::domain::adjacency::types::compact_graph::value::CompactGraph;
use crate::domain::allocation::types::ElementCapacity;
use crate::domain::closure_adjacency::types::ClosurePartition;
use crate::domain::cycle_adjacency::types::CycleGraph;
use crate::domain::dfs::types::PreparedDfs;
use crate::domain::equality::types::KeyIndex;
use crate::domain::membership::types::DerivedMembership;
use crate::domain::native::types::NativeFinding;
use crate::domain::ordered::types::OrderedMatching;
use crate::domain::predicate::types::LiteralPostings;
use crate::domain::raw_order::types::raw_order::value::RawOrder;
use crate::domain::sharing::types::{SharedGraph, SharedKeyIndex, SharedMarks, SharedPositions};
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::rows::types::RowIndex;
use std::sync::Arc;

/// Successful payload construction is private to the capability's build dispatcher.
///
/// A caller cannot construct or rewrite a payload.
///
/// ```compile_fail,E0451
/// use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
/// fn forge() -> DerivedPayload { DerivedPayload { data: todo!() } }
/// ```
///
/// A caller cannot rewrite a payload it only borrows.
///
/// ```compile_fail,E0594
/// use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
/// fn rewrite(borrowed: &DerivedPayload, other: DerivedPayload) { *borrowed = other; }
/// ```
#[derive(Debug)]
pub struct DerivedPayload {
    pub(crate) data: PayloadData,
}

/// What each kind of payload holds.
#[derive(Debug)]
pub(crate) enum PayloadData {
    Equality(Arc<KeyIndex>),
    PredicateMatches(SharedPositions),
    CandidatePostings(Vec<LiteralPostings>),
    DistinctRepresentatives(SharedPositions),
    Adjacency(Arc<CompactGraph>),
    Membership(DerivedMembership),
    PartitionOrder(Vec<RowIndex>),
    ClosureAdjacency(Arc<ClosurePartition>),
    CycleAdjacency(Arc<CycleGraph>),
    OrderedKeys(Arc<OrderedMatching>),
    OrderedDfs(Arc<PreparedDfs>),
    NativeLookup(NativeFinding),
    RawKeyOrder(Arc<RawOrder>),
}

impl DerivedPayload {
    /// Capacity of the immutable typed ordering allocation, without lending its ownership.
    #[must_use]
    pub fn partition_order_capacity(&self) -> Option<ElementCapacity> {
        match &self.data {
            PayloadData::PartitionOrder(rows) => Some(ElementCapacity(rows.capacity())),
            _ => None,
        }
    }
    /// The shared positions of a predicate or distinct payload.
    #[must_use]
    pub fn share_positions(&self) -> Option<SharedPositions> {
        match &self.data {
            PayloadData::PredicateMatches(rows) | PayloadData::DistinctRepresentatives(rows) => {
                Some(rows.clone())
            }
            _ => None,
        }
    }
    /// The shared positions of one literal of a postings payload.
    #[must_use]
    pub fn share_posting_positions(&self, text: &LiteralText) -> Option<SharedPositions> {
        let PayloadData::CandidatePostings(values) = &self.data else {
            return None;
        };
        let Ok(at) = values.binary_search_by(|entry| entry.text.0.cmp(&text.0)) else {
            return None;
        };
        values.get(at).map(|entry| entry.rows.clone())
    }
    /// The shared marks of a dense membership payload.
    #[must_use]
    pub fn share_marks(&self) -> Option<SharedMarks> {
        match &self.data {
            PayloadData::Membership(DerivedMembership::Dense(rows)) => Some(rows.clone()),
            _ => None,
        }
    }
    /// The shared key index of an equality payload.
    #[must_use]
    pub fn share_equality(&self) -> Option<SharedKeyIndex> {
        match &self.data {
            PayloadData::Equality(data) => Some(SharedKeyIndex(Arc::clone(data))),
            _ => None,
        }
    }
    /// The shared graph of an adjacency payload.
    #[must_use]
    pub fn share_adjacency(&self) -> Option<SharedGraph> {
        match &self.data {
            PayloadData::Adjacency(data) => Some(SharedGraph(Arc::clone(data))),
            _ => None,
        }
    }
}
