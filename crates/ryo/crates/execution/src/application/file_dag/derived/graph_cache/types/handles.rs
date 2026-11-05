//! Required invocation leases travel with the immutable graph/domain handles.
use crate::application::file_dag::derived::ledger::Backing;
use crate::application::file_dag::derived::ledger::ResourceLease;
use crate::application::file_dag::derived::owner::{GraphRequirement, PopulationId};
use crate::application::file_dag::derived::storage::DerivedStorage;
use crate::domain::stored_graph::types::{GraphDomain, Orientation};
use ryo_derived::domain::contract::types::CompactGraph;
use std::{ops::Deref, sync::Arc};

#[derive(Debug)]
pub(in crate::application::file_dag) struct GraphAccess {
    /// Complete immutable adjacency, retained or invocation-local.
    pub graph: Arc<CompactGraph>,
    /// Original requested source relative to canonical endpoint order.
    pub orientation: Orientation,
    /// Whether the generation owns this adjacency allocation.
    pub shared: bool,
}

pub(in crate::application::file_dag) struct GraphHandle {
    pub(in crate::application::file_dag::derived) requirement: GraphRequirement,
    pub(in crate::application::file_dag::derived) population: PopulationId,
    pub(in crate::application::file_dag::derived) generation: Arc<()>,
    pub(in crate::application::file_dag::derived) built: bool,
    pub(in crate::application::file_dag::derived) access: GraphAccess,
    pub(in crate::application::file_dag::derived) _storage: Arc<DerivedStorage>,
    pub(in crate::application::file_dag::derived) backings: Arc<[Backing]>,
    pub(in crate::application::file_dag::derived) _lease: Arc<ResourceLease>,
}
impl Deref for GraphHandle {
    type Target = GraphAccess;
    fn deref(&self) -> &GraphAccess {
        &self.access
    }
}
pub(in crate::application::file_dag) struct DomainHandle {
    pub(in crate::application::file_dag::derived) _graph: Arc<DerivedStorage>,
    pub(in crate::application::file_dag::derived) built: bool,
    pub(in crate::application::file_dag::derived) domain: Arc<GraphDomain>,
    pub(in crate::application::file_dag::derived) _storage: Arc<DerivedStorage>,
    pub(in crate::application::file_dag::derived) _lease: Arc<ResourceLease>,
}
impl Deref for DomainHandle {
    type Target = Arc<GraphDomain>;
    fn deref(&self) -> &Arc<GraphDomain> {
        &self.domain
    }
}

/// A completed dense answer keeps both its immutable source space and its
/// required lease; callers cannot detach the Arc payload from that lease.
#[derive(Debug)]
pub(in crate::application::file_dag) struct RootAnswer {
    rows: Arc<Vec<u32>>,
    _graph: Arc<DerivedStorage>,
    _lease: Arc<ResourceLease>,
}
impl RootAnswer {
    pub(in crate::application::file_dag::derived) fn new(
        rows: Arc<Vec<u32>>,
        graph: Arc<DerivedStorage>,
        lease: Arc<ResourceLease>,
    ) -> Self {
        Self {
            rows,
            _graph: graph,
            _lease: lease,
        }
    }
    /// Decode in the exact immutable dense space that completed this answer.
    pub(in crate::application::file_dag) fn values(
        &self,
    ) -> Result<
        impl Iterator<
            Item = Result<
                ryo_relations::domain::relations::types::Value,
                crate::domain::operating::errors::OperatingError,
            >,
        > + '_,
        crate::domain::operating::errors::OperatingError,
    > {
        use crate::domain::operating::errors::OperatingError;
        let ryo_derived::domain::contract::types::DerivedView::Adjacency(graph) =
            self._graph.view()?
        else {
            return Err(OperatingError::WrongOperation);
        };
        Ok(self.rows.iter().map(move |vertex| {
            graph
                .vertices()
                .get(*vertex as usize)
                .copied()
                .ok_or(OperatingError::ShapeMismatch)
        }))
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn same_data(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.rows, &other.rows)
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn weak(&self) -> std::sync::Weak<Vec<u32>> {
        Arc::downgrade(&self.rows)
    }
}
impl Deref for RootAnswer {
    type Target = Vec<u32>;
    fn deref(&self) -> &Vec<u32> {
        &self.rows
    }
}
