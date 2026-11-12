//! The built payload of one derived kind: the value and its behaviour, owned together.
use crate::domain::adjacency::types::compact_graph::value::CompactGraph;
use crate::domain::allocation::types::ElementCapacity;
use crate::domain::closure_adjacency::types::ClosurePartition;
use crate::domain::contract::types::DerivedView;
use crate::domain::cycle_adjacency::types::CycleGraph;
use crate::domain::dfs::types::PreparedDfs;
use crate::domain::equality::types::KeyIndex;
use crate::domain::membership::types::{DerivedMembership, MembershipView};
use crate::domain::native::types::NativeFinding;
use crate::domain::ordered::types::OrderedMatching;
use crate::domain::positions::types::DenseMarks;
use crate::domain::sharing::types::{SharedGraph, SharedKeyIndex, SharedMarks};
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
    Adjacency(Arc<CompactGraph>),
    Membership(DerivedMembership),
    PartitionOrder(Vec<RowIndex>),
    ClosureAdjacency(Arc<ClosurePartition>),
    CycleAdjacency(Arc<CycleGraph>),
    OrderedKeys(Arc<OrderedMatching>),
    OrderedDfs(Arc<PreparedDfs>),
    NativeLookup(NativeFinding),
}

impl DerivedPayload {
    /// Capacity of the immutable typed ordering allocation, without lending its ownership.
    #[must_use]
    pub fn partition_order_capacity(&self) -> Option<ElementCapacity> {
        if let PayloadData::PartitionOrder(rows) = &self.data {
            Some(ElementCapacity(rows.capacity()))
        } else {
            None
        }
    }
    /// The shared marks of a dense membership payload.
    #[must_use]
    pub fn share_marks(&self) -> Option<SharedMarks> {
        if let PayloadData::Membership(DerivedMembership::Dense(rows)) = &self.data {
            Some(rows.clone())
        } else {
            None
        }
    }
    /// The shared key index of an equality payload.
    #[must_use]
    pub fn share_equality(&self) -> Option<SharedKeyIndex> {
        if let PayloadData::Equality(data) = &self.data {
            Some(SharedKeyIndex(Arc::clone(data)))
        } else {
            None
        }
    }
    /// The shared graph of an adjacency payload.
    #[must_use]
    pub fn share_adjacency(&self) -> Option<SharedGraph> {
        if let PayloadData::Adjacency(data) = &self.data {
            Some(SharedGraph(Arc::clone(data)))
        } else {
            None
        }
    }
    /// The view of this payload, of whatever kind it was built: the view a prepared request borrows, without the request.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_derived::domain::contract::types::DerivedView;
    /// use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
    /// fn is_index(payload: &DerivedPayload) -> bool {
    ///     matches!(payload.view(), DerivedView::Equality(_))
    /// }
    /// let _ = is_index;
    /// ```
    #[must_use]
    pub fn view(&self) -> DerivedView<'_> {
        match &self.data {
            PayloadData::Equality(data) => DerivedView::Equality(data),
            PayloadData::Adjacency(data) => DerivedView::Adjacency(data),
            PayloadData::Membership(DerivedMembership::Values(values)) => {
                DerivedView::Membership(MembershipView::Values(values))
            }
            PayloadData::Membership(DerivedMembership::Dense(marks)) => {
                DerivedView::Membership(MembershipView::Dense(DenseMarks(marks.0.as_slice())))
            }
            PayloadData::PartitionOrder(data) => DerivedView::PartitionOrder(data),
            PayloadData::ClosureAdjacency(data) => DerivedView::ClosureAdjacency(data),
            PayloadData::CycleAdjacency(data) => DerivedView::CycleAdjacency(data),
            PayloadData::OrderedKeys(data) => DerivedView::OrderedKeys(data),
            PayloadData::OrderedDfs(data) => DerivedView::OrderedDfs(data),
            PayloadData::NativeLookup(data) => DerivedView::NativeLookup(data),
        }
    }
}
