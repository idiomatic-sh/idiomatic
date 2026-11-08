//! Retained graph storage belongs to the application owner, never semantic algorithms.
use crate::application::file_dag::derived::ledger::types::Backing;
use crate::application::file_dag::derived::ledger::types::ResourceLease;
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage;
use crate::application::file_dag::derived::population::types::PopulationId;
use ryo_execution_domain::domain::stored_graph::types::{GraphDomain, Orientation};
use ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph;
use ryo_derived::domain::vertices::types::VertexId;
use std::sync::Arc;
/// The dense vertices a completed closure reached, in typed value order: the bare numbers inside are named here.
#[derive(Debug)]
pub(crate) struct ReachedVertices(pub Vec<u32>);

/// The index of one nearest memo among the retained memos: the bare index inside is named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NearestSlot(pub usize);

#[derive(Debug)]
pub(crate) struct RootMemo {
    /// Dense root already identifies its partition.
    pub(crate) root: VertexId,
    /// Requested traversal orientation.
    pub(crate) orientation: Orientation,
    /// Reachable dense IDs in typed Value order.
    pub(crate) reached: Arc<ReachedVertices>,
    pub(crate) _lease: Arc<ResourceLease>,
}

#[derive(Debug)]
pub(crate) struct GraphEntry {
    /// Immutable handle may be held across nested emit after mutable cache access ends.
    pub(crate) graph: Arc<CompactGraph>,
    /// Successful source requests only; never a full all-roots closure.
    pub(crate) roots: Vec<RootMemo>,
    pub(crate) population: PopulationId,
    pub(crate) storage: Arc<DerivedStorage>,
    pub(crate) backings: Arc<[Backing]>,
    pub(crate) _lease: Arc<ResourceLease>,
}

#[derive(Debug)]
pub(crate) struct DomainEntry {
    pub(crate) graph_storage: Arc<DerivedStorage>,
    /// Adjacency whose dense IDs the membership vector uses.
    pub(crate) graph: Arc<CompactGraph>,
    /// Exact membership context.
    pub(crate) domain: Arc<GraphDomain>,
    pub(crate) population: PopulationId,
    pub(crate) storage: Arc<DerivedStorage>,
    pub(crate) backings: Arc<[Backing]>,
    pub(crate) _lease: Arc<ResourceLease>,
}

#[derive(Debug)]
pub(crate) struct NearestMemo {
    pub(crate) _graph_storage: Arc<DerivedStorage>,
    pub(crate) _domain_storage: Arc<DerivedStorage>,
    /// Exact adjacency context.
    pub(crate) graph: Arc<CompactGraph>,
    /// Exact membership context.
    pub(crate) domain: Arc<GraphDomain>,
    /// Parent direction whose forest was validated.
    pub(crate) orientation: Orientation,
    /// Unknown/absent/answer sentinels or dense IDs, never public values.
    pub(crate) answers: Vec<VertexId>,
    pub(crate) _lease: Arc<ResourceLease>,
}
