//! The `program` application operation: the request's derived access, held for a prepared program.
use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
use ryo_execution_store::application::file_dag::derived::request::service::RequestViews;
use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
use ryo_execution_store::application::file_dag::relations::types::{
    ProducerCapacity, ReuseEligibility,
};
use ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile;
use ryo_execution_store::application::file_dag::derived::relation_owner::service::open_program_owner;
use ryo_execution_store::outbound::route_counts::interface::RouteCountsPort;
use std::rc::Rc;
use std::sync::Arc;

/// The request's derived views as a prepared program uses them: it binds each operator's derived views once, and opens
/// the relation owner of every file and project scope. It lends no access and acquires no view.
#[derive(Clone, Debug)]
pub(crate) struct ProgramDerived(DerivedAccess);

impl<T: Into<DerivedAccess>> From<T> for ProgramDerived {
    fn from(access: T) -> Self {
        Self(access.into())
    }
}

impl ProgramDerived {
    /// The views that bind the derived slots of the request's operators, each prepared once for the request.
    pub(crate) fn request_views(&self) -> RequestViews {
        RequestViews::new(self.0.clone())
    }
    /// The relation owner of one scope: room for `producers`, reused as `reuse` allows, its work recorded into `routes`.
    pub(crate) fn open_owner(
        &self,
        producers: ProducerCapacity,
        reuse: ReuseEligibility,
        routes: RouteProfile,
    ) -> Rc<RelationOwner> {
        open_program_owner(producers, self.0.clone(), reuse, routes)
    }
    /// The format route accounting writes in, when the edge composed one.
    pub(crate) fn routes(&self) -> Option<Arc<dyn RouteCountsPort>> {
        self.0.routes()
    }
}
