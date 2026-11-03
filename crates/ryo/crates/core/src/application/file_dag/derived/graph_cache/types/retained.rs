//! Retained graph storage belongs to the application owner, never semantic algorithms.
use crate::application::file_dag::derived::ledger::Backing;
use crate::application::file_dag::derived::ledger::ResourceLease;
use crate::application::file_dag::derived::owner::PopulationId;
use crate::application::file_dag::derived::storage::DerivedStorage;
use crate::domain::stored_graph::types::{GraphDomain, Orientation};
use ryo_derived::domain::contract::types::CompactGraph;
use std::sync::Arc;
#[derive(Debug)]
pub(crate) struct RootMemo {
    /// Dense root already identifies its partition.
    pub root: u32,
    /// Requested traversal orientation.
    pub orientation: Orientation,
    /// Reachable dense IDs in typed Value order.
    pub reached: Arc<Vec<u32>>,
    pub _lease: Arc<ResourceLease>,
}

#[derive(Debug)]
pub(crate) struct GraphEntry {
    /// Immutable handle may be held across nested emit after mutable cache access ends.
    pub graph: Arc<CompactGraph>,
    /// Successful source requests only; never a full all-roots closure.
    pub roots: Vec<RootMemo>,
    pub population: PopulationId,
    pub storage: Arc<DerivedStorage>,
    pub backings: Arc<[Backing]>,
    pub _lease: Arc<ResourceLease>,
}

#[derive(Debug)]
pub(crate) struct DomainEntry {
    pub graph_storage: Arc<DerivedStorage>,
    /// Adjacency whose dense IDs the membership vector uses.
    pub graph: Arc<CompactGraph>,
    /// Exact membership context.
    pub domain: Arc<GraphDomain>,
    pub population: PopulationId,
    pub storage: Arc<DerivedStorage>,
    pub backings: Arc<[Backing]>,
    pub _lease: Arc<ResourceLease>,
}

#[derive(Debug)]
pub(crate) struct NearestMemo {
    pub _graph_storage: Arc<DerivedStorage>,
    pub _domain_storage: Arc<DerivedStorage>,
    /// Exact adjacency context.
    pub graph: Arc<CompactGraph>,
    /// Exact membership context.
    pub domain: Arc<GraphDomain>,
    /// Parent direction whose forest was validated.
    pub orientation: Orientation,
    /// Unknown/absent/answer sentinels or dense IDs, never public values.
    pub answers: Vec<u32>,
    pub _lease: Arc<ResourceLease>,
}
