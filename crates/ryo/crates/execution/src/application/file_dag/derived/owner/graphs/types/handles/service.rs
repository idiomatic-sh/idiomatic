//! Required invocation leases travel with the immutable graph/domain handles.
#[cfg(test)]
use super::types::DataIdentity;
use super::types::GraphSharing;
#[cfg(test)]
use crate::application::file_dag::derived::constants;
use crate::application::file_dag::derived::ledger::types::Backing;
use crate::application::file_dag::derived::ledger::types::ResourceLease;
use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage;
use crate::domain::stored_graph::types::{GraphDomain, Orientation};
use ryo_derived::domain::contract::types::CompactGraph;
use std::{ops::Deref, sync::Arc};

/// An actual completed adjacency and its required lease, issued together.
pub(in crate::application::file_dag) struct GraphHandle {
    graph: Arc<CompactGraph>,
    orientation: Orientation,
    sharing: GraphSharing,
    acquired: AcquiredView,
}
impl GraphHandle {
    pub(in crate::application::file_dag::derived::owner) fn new(
        owner: &crate::application::file_dag::derived::owner::DerivedOwner,
        acquired: AcquiredView,
        orientation: Orientation,
        sharing: GraphSharing,
    ) -> Result<Self, crate::domain::operating::errors::OperatingError> {
        acquired.validate_owner(owner)?;
        let graph = acquired
            .storage()
            .payload()
            .share_adjacency()
            .ok_or(crate::domain::operating::errors::OperatingError::WrongOperation)?;
        Ok(Self {
            graph,
            orientation,
            sharing,
            acquired,
        })
    }
    pub(in crate::application::file_dag) fn graph(&self) -> &CompactGraph {
        #[cfg(test)]
        self.acquired.view().expect(constants::EXPECT_GRAPH_LEASE);
        &self.graph
    }
    pub(in crate::application::file_dag) fn orientation(&self) -> Orientation {
        self.orientation
    }
    pub(in crate::application::file_dag::derived::owner) fn sharing(&self) -> GraphSharing {
        self.sharing
    }
    pub(in crate::application::file_dag::derived::owner) fn raw_graph(&self) -> &Arc<CompactGraph> {
        &self.graph
    }
    pub(in crate::application::file_dag::derived::owner) fn acquired(&self) -> &AcquiredView {
        &self.acquired
    }
    pub(in crate::application::file_dag::derived::owner) fn backings(&self) -> &Arc<[Backing]> {
        self.acquired.backings()
    }
    pub(in crate::application::file_dag) fn validate_owner(
        &self,
        owner: &crate::application::file_dag::derived::owner::DerivedOwner,
    ) -> Result<(), crate::domain::operating::errors::OperatingError> {
        self.acquired.validate_owner(owner)
    }
}
/// Membership never exposes a cloneable graph/domain allocation to a consumer.
pub(in crate::application::file_dag) struct DomainHandle {
    acquired: AcquiredView,
    parent: AcquiredView,
    domain: Arc<GraphDomain>,
    _lease: Arc<ResourceLease>,
}
impl DomainHandle {
    pub(in crate::application::file_dag::derived::owner) fn new(
        owner: &crate::application::file_dag::derived::owner::DerivedOwner,
        parent: &GraphHandle,
        acquired: AcquiredView,
        domain: Arc<GraphDomain>,
        lease: Arc<ResourceLease>,
    ) -> Result<Self, crate::domain::operating::errors::OperatingError> {
        use crate::domain::operating::errors::OperatingError;
        parent.validate_owner(owner)?;
        acquired.validate_owner(owner)?;
        let marks = acquired
            .storage()
            .payload()
            .share_marks()
            .ok_or(OperatingError::WrongOperation)?;
        if !Arc::ptr_eq(marks.backing(), domain.members.backing())
            || marks.as_slice().len() != parent.graph().vertices().len()
        {
            return Err(OperatingError::NotRead);
        }
        Ok(Self {
            acquired,
            parent: parent.acquired.clone(),
            domain,
            _lease: lease,
        })
    }
    pub(in crate::application::file_dag) fn members(&self) -> &[bool] {
        #[cfg(test)]
        self.acquired
            .view()
            .expect(constants::EXPECT_MEMBERSHIP_LEASE);
        self.domain.members.as_slice()
    }
    pub(in crate::application::file_dag::derived::owner) fn raw_domain(&self) -> &Arc<GraphDomain> {
        &self.domain
    }
    pub(in crate::application::file_dag) fn validate_owner(
        &self,
        owner: &crate::application::file_dag::derived::owner::DerivedOwner,
        graph: &GraphHandle,
    ) -> Result<(), crate::domain::operating::errors::OperatingError> {
        self.acquired.validate_owner(owner)?;
        self.parent.validate_owner(owner)?;
        if !Arc::ptr_eq(self.parent.storage(), graph.acquired.storage()) {
            return Err(crate::domain::operating::errors::OperatingError::NotRead);
        }
        Ok(())
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
    pub(in crate::application::file_dag::derived::owner) fn new(
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
    pub(in crate::application::file_dag) fn data_identity(&self) -> DataIdentity {
        DataIdentity(Arc::as_ptr(&self.rows).cast())
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
