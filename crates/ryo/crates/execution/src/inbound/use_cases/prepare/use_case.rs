//! Thin public orchestration over the one private execution owner.
use super::interface::PrepareUseCase;
use super::types::{PlanSource, PrepareRequest};
use crate::application::file_dag::derived::owner::types::DerivedAccess;
use crate::application::file_dag::program::prepared_execution::service::prepare;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::domain::executable_program::types::RoutePolicy;
use crate::domain::operating::errors::OperatingError;
use crate::outbound::derived::interface::DerivedPort;
use crate::outbound::operators::interface::OperatorsPort;
use std::sync::Arc;

/// Prepares programs over the operators and derived views it is given.
pub struct PrepareUseCaseImpl {
    operators: Arc<dyn OperatorsPort>,
    derived: DerivedAccess,
    routes: RoutePolicy,
}

impl PrepareUseCaseImpl {
    /// A use case over the engine's operators and derived views.
    pub fn new(operators: Arc<dyn OperatorsPort>, derived: Arc<dyn DerivedPort>) -> Self {
        Self {
            operators,
            derived: derived.into(),
            routes: RoutePolicy::default(),
        }
    }
    /// The same engine wiring only the strategies `routes` names, for a differential run.
    #[cfg(any(test, feature = "fixtures"))]
    #[must_use]
    pub const fn routing(mut self, routes: RoutePolicy) -> Self {
        self.routes = routes;
        self
    }
}

impl PrepareUseCase for PrepareUseCaseImpl {
    fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError> {
        let compiled = match request.plans {
            PlanSource::Authored { plans, bindings } => {
                crate::domain::prepared_request::service::assemble(plans, bindings, &request.rules)?
            }
            PlanSource::Compiled(queries) => queries.inner,
        };
        prepare(
            compiled,
            &request.rules,
            &request.digests,
            &request.hosted,
            request.observations,
            Arc::clone(&self.operators),
            &self.derived,
            self.routes,
        )
    }
}
