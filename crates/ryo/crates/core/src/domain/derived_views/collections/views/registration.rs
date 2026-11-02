//! One closed inventory; no implementation may opt out or select a legacy owner.
use super::{interface::DerivedViewsItem, items};
use crate::domain::derived_views::types::DerivedKind;
static ITEMS: [&dyn DerivedViewsItem; DerivedKind::ALL.len()] = [
    &items::EqualityItem,
    &items::PredicateMatchesItem,
    &items::CandidatePostingsItem,
    &items::DistinctRepresentativesItem,
    &items::AdjacencyItem,
    &items::MembershipItem,
    &items::PartitionOrderItem,
    &items::ClosureAdjacencyItem,
    &items::CycleAdjacencyItem,
    &items::OrderedKeysItem,
    &items::OrderedDfsItem,
    &items::NativeLookupItem,
];
pub(crate) fn registered(kind: DerivedKind) -> &'static dyn DerivedViewsItem {
    match kind {
        DerivedKind::Equality => ITEMS[0],
        DerivedKind::PredicateMatches => ITEMS[1],
        DerivedKind::CandidatePostings => ITEMS[2],
        DerivedKind::DistinctRepresentatives => ITEMS[3],
        DerivedKind::Adjacency => ITEMS[4],
        DerivedKind::Membership => ITEMS[5],
        DerivedKind::PartitionOrder => ITEMS[6],
        DerivedKind::ClosureAdjacency => ITEMS[7],
        DerivedKind::CycleAdjacency => ITEMS[8],
        DerivedKind::OrderedKeys => ITEMS[9],
        DerivedKind::OrderedDfs => ITEMS[10],
        DerivedKind::NativeLookup => ITEMS[11],
    }
}
