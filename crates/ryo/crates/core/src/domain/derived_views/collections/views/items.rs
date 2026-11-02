//! Actual semantic items; one exhaustive registry owns their complete inventory.
pub(crate) use super::key::EqualityItem;

pub(crate) use super::predicate::{
    CandidatePostingsItem, DistinctRepresentativesItem, PredicateMatchesItem,
};

pub(crate) use super::membership::MembershipItem;
pub(crate) use super::native::NativeLookupItem;

pub(crate) use super::graph::{
    AdjacencyItem, ClosureAdjacencyItem, CycleAdjacencyItem, OrderedDfsItem, OrderedKeysItem,
    PartitionOrderItem,
};
