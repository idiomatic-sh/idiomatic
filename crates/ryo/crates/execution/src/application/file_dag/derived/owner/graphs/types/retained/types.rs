//! Retained graph storage belongs to the application owner, never semantic algorithms.
use crate::application::file_dag::derived::ledger::types::Backing;
use crate::application::file_dag::derived::ledger::types::ResourceLease;
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage;
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::domain::stored_graph::types::{GraphDomain, Orientation, VertexId};
use ryo_derived::domain::contract::types::CompactGraph;
use std::sync::Arc;
#[derive(Debug)]
pub(in crate::application::file_dag::derived::owner) struct RootMemo {
    /// Dense root already identifies its partition.
    pub(in crate::application::file_dag::derived::owner) root: u32,
    /// Requested traversal orientation.
    pub(in crate::application::file_dag::derived::owner) orientation: Orientation,
    /// Reachable dense IDs in typed Value order.
    pub(in crate::application::file_dag::derived::owner) reached: Arc<Vec<u32>>,
    pub(in crate::application::file_dag::derived::owner) _lease: Arc<ResourceLease>,
}

#[derive(Debug)]
pub(in crate::application::file_dag::derived::owner) struct GraphEntry {
    /// Immutable handle may be held across nested emit after mutable cache access ends.
    pub(in crate::application::file_dag::derived::owner) graph: Arc<CompactGraph>,
    /// Successful source requests only; never a full all-roots closure.
    pub(in crate::application::file_dag::derived::owner) roots: Vec<RootMemo>,
    pub(in crate::application::file_dag::derived::owner) population: PopulationId,
    pub(in crate::application::file_dag::derived::owner) storage: Arc<DerivedStorage>,
    pub(in crate::application::file_dag::derived::owner) backings: Arc<[Backing]>,
    pub(in crate::application::file_dag::derived::owner) _lease: Arc<ResourceLease>,
}

#[derive(Debug)]
pub(in crate::application::file_dag::derived::owner) struct DomainEntry {
    pub(in crate::application::file_dag::derived::owner) graph_storage: Arc<DerivedStorage>,
    /// Adjacency whose dense IDs the membership vector uses.
    pub(in crate::application::file_dag::derived::owner) graph: Arc<CompactGraph>,
    /// Exact membership context.
    pub(in crate::application::file_dag::derived::owner) domain: Arc<GraphDomain>,
    pub(in crate::application::file_dag::derived::owner) population: PopulationId,
    pub(in crate::application::file_dag::derived::owner) storage: Arc<DerivedStorage>,
    pub(in crate::application::file_dag::derived::owner) backings: Arc<[Backing]>,
    pub(in crate::application::file_dag::derived::owner) _lease: Arc<ResourceLease>,
}

#[derive(Debug)]
pub(in crate::application::file_dag::derived::owner) struct NearestMemo {
    pub(in crate::application::file_dag::derived::owner) _graph_storage: Arc<DerivedStorage>,
    pub(in crate::application::file_dag::derived::owner) _domain_storage: Arc<DerivedStorage>,
    /// Exact adjacency context.
    pub(in crate::application::file_dag::derived::owner) graph: Arc<CompactGraph>,
    /// Exact membership context.
    pub(in crate::application::file_dag::derived::owner) domain: Arc<GraphDomain>,
    /// Parent direction whose forest was validated.
    pub(in crate::application::file_dag::derived::owner) orientation: Orientation,
    /// Unknown/absent/answer sentinels or dense IDs, never public values.
    pub(in crate::application::file_dag::derived::owner) answers: Vec<VertexId>,
    pub(in crate::application::file_dag::derived::owner) _lease: Arc<ResourceLease>,
}
