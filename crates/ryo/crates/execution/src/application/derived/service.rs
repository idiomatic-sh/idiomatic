//! The derived access as execution holds it, between the edge that composes it and each request it prepares: it lends no
//! access and acquires no view, it only prepares a request's program.
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::{Compiled, ObservationIdentity};
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
use ryo_execution_store::outbound::derived::interface::DerivedPort;
use ryo_execution_store::outbound::route_counts::interface::RouteCountsPort;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;

/// The derived views a request's program is prepared over, held so that it only prepares.
#[derive(Clone, Debug)]
pub(crate) struct DerivedPreparation(pub(super) DerivedAccess);

impl DerivedPreparation {
    /// The preparation over the derived `port`, whose route accounting writes its documents through `routes`.
    #[must_use]
    pub(crate) fn routed(port: Arc<dyn DerivedPort>, routes: Arc<dyn RouteCountsPort>) -> Self {
        Self(DerivedAccess::from(port).routed(routes))
    }

    /// The program of a request, prepared over the engine's `operators` and `digest` and the derived views held.
    ///
    /// # Errors
    ///
    /// The refusal of preparing the request's program: rules and digests that do not pair, an operator the domain
    /// cannot prepare or whose derived views cannot be bound, and an incomplete named observation request.
    pub(crate) fn program(
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
