//! The `derived` application operation: the request's access to the derived views, as the runtime holds it, and the
//! file preparation of the derived owner.
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_execution_domain::domain::coverage::types::{Partial, RuleIndex};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
use ryo_execution_store::application::file_dag::derived::prepared::service::RequestDerived;
use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
use ryo_execution_store::outbound::derived::interface::DerivedPort;
use ryo_execution_store::outbound::route_counts::interface::RouteCountsPort;
use ryo_relations::domain::relations::types::CellKind;
use std::sync::Arc;

/// The request's access to the derived views, as the runtime holds it: a program is prepared over it, and every file's
/// derived owner is opened with it.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::derived::service::ViewAccess;
/// fn shared(access: &ViewAccess) -> ViewAccess {
///     access.clone()
/// }
/// ```
#[derive(Clone, Debug)]
pub struct ViewAccess(DerivedAccess);

impl ViewAccess {
    /// The access over the derived `port`, whose route accounting writes its documents through `routes`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_runtime::application::file_dag::derived::service::ViewAccess;
    /// use std::sync::Arc;
    ///
    /// let access = ViewAccess::routed(
    ///     Arc::new(ryo_execution_store::outbound::derived::factory::use_case(
    ///         ryo_execution_store::outbound::derived::factory::semantics(),
    ///     )),
    ///     Arc::new(ryo_execution_store::outbound::route_counts::factory::json()),
    /// );
    /// ```
    #[must_use]
    pub fn routed(port: Arc<dyn DerivedPort>, routes: Arc<dyn RouteCountsPort>) -> Self {
        Self(DerivedAccess::from(port).routed(routes))
    }
    /// The derived contract `request` plans over `schemas`: request-time metadata only, which never builds or retains a
    /// view.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_derived::domain::request::types::DerivedRequest;
    /// use ryo_execution_runtime::application::file_dag::derived::service::ViewAccess;
    ///
    /// fn prepared(access: &ViewAccess, request: DerivedRequest) {
    ///     let _contract = access.prepare(request, Vec::new());
    /// }
    /// ```
    #[must_use]
    pub fn prepare(
        &self,
        request: DerivedRequest,
        schemas: Vec<Vec<CellKind>>,
    ) -> Arc<PreparedDerived> {
        self.0.prepare(request, schemas)
    }
    /// The format route accounting writes in, when the edge composed one.
    pub(crate) fn routes(&self) -> Option<Arc<dyn RouteCountsPort>> {
        self.0.routes()
    }
}

impl From<DerivedAccess> for ViewAccess {
    fn from(access: DerivedAccess) -> Self {
        Self(access)
    }
}

impl From<&DerivedAccess> for ViewAccess {
    fn from(access: &DerivedAccess) -> Self {
        Self(access.clone())
    }
}

impl From<Arc<dyn DerivedPort>> for ViewAccess {
    fn from(port: Arc<dyn DerivedPort>) -> Self {
        Self(DerivedAccess::from(port))
    }
}

impl<T: DerivedPort + 'static> From<Arc<T>> for ViewAccess {
    fn from(port: Arc<T>) -> Self {
        Self(DerivedAccess::from(port))
    }
}

impl From<ViewAccess> for DerivedAccess {
    fn from(access: ViewAccess) -> Self {
        access.0
    }
}

/// Prepares the file owner's derived views for the rules the file demands, over the request's prepared derived needs.
pub(crate) fn prepare_file(
    owner: &RelationOwner,
    prepared: &RequestDerived,
    demanded: &[RuleIndex],
    partial: &[Partial],
) -> Result<(), OperatingError> {
    owner
        .derived()
        .prepare_program_file(prepared, demanded, partial)
}
