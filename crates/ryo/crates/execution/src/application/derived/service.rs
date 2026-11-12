//! The derived access as execution holds it, between the edge that composes it and each request it prepares: it lends no
//! access and acquires no view, it only prepares a request's program and a derived contract.
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::{Compiled, ObservationIdentity};
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
use ryo_execution_store::outbound::derived::interface::DerivedPort;
use ryo_execution_store::outbound::route_counts::interface::RouteCountsPort;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;

/// The derived views a request's program is prepared over, held so that it only prepares.
///
/// # Examples
///
/// ```
/// use ryo_execution::application::derived::service::DerivedPreparation;
/// use std::sync::Arc;
///
/// let preparation = DerivedPreparation::from(Arc::new(
///     ryo_execution_store::outbound::derived::factory::semantics(),
/// ));
/// let shared: DerivedPreparation = preparation.clone();
/// ```
#[derive(Clone, Debug)]
pub struct DerivedPreparation(DerivedAccess);

impl DerivedPreparation {
    /// The preparation over the derived `port`, whose route accounting writes its documents through `routes`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution::application::derived::service::DerivedPreparation;
    /// use std::sync::Arc;
    ///
    /// let preparation = DerivedPreparation::routed(
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

    /// The program of a request, prepared over the engine's `operators` and `digest` and the derived views held.
    ///
    /// # Errors
    ///
    /// The refusal of preparing the request's program: rules and digests that do not pair, an operator the domain
    /// cannot prepare or whose derived views cannot be bound, and an incomplete named observation request.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution::application::derived::service::DerivedPreparation;
    /// use std::sync::Arc;
    ///
    /// let preparation = DerivedPreparation::from(Arc::new(
    ///     ryo_execution_store::outbound::derived::factory::semantics(),
    /// ));
    /// let _ = DerivedPreparation::program;
    /// ```
    pub fn program(
        &self,
        compiled: Compiled,
        rules: &[RuleSpec],
        digests: &[Digest],
        hosted: &[HostRelation],
        identity: ObservationIdentity,
        operators: Arc<dyn OperatorsPort>,
        digest: Arc<dyn DigestPort>,
    ) -> Result<PreparedExecutionProgram, OperatingError> {
        ryo_execution_runtime::application::file_dag::derived::service::prepare(
            compiled,
            rules,
            digests,
            hosted,
            identity,
            operators,
            self.0.clone(),
            digest,
        )
    }

    /// The derived contract `request` plans over `schemas`: request-time metadata only, which never builds or retains a
    /// view.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_derived::domain::request::types::DerivedRequest;
    /// use ryo_execution::application::derived::service::DerivedPreparation;
    ///
    /// fn prepared(preparation: &DerivedPreparation, request: DerivedRequest) {
    ///     let _contract = preparation.contract(request, Vec::new());
    /// }
    /// ```
    #[must_use]
    pub fn contract(
        &self,
        request: DerivedRequest,
        schemas: Vec<Vec<CellKind>>,
    ) -> PreparedDerived {
        Arc::unwrap_or_clone(self.0.prepare(request, schemas))
    }
}

impl From<Arc<dyn DerivedPort>> for DerivedPreparation {
    fn from(port: Arc<dyn DerivedPort>) -> Self {
        Self(DerivedAccess::from(port))
    }
}

impl<T: DerivedPort + 'static> From<Arc<T>> for DerivedPreparation {
    fn from(port: Arc<T>) -> Self {
        Self(DerivedAccess::from(port))
    }
}
