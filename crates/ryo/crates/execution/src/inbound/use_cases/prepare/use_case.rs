//! Thin public orchestration over the one private execution owner.
use super::interface::PrepareUseCase;
use super::types::{PlanSource, PrepareRequest};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::service::prepare;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
use std::sync::Arc;

/// Prepares programs over the operators, derived views and digest it is given.
pub struct PrepareUseCaseImpl {
    operators: Arc<dyn OperatorsPort>,
    derived: DerivedAccess,
    digest: Arc<dyn DigestPort>,
}

impl PrepareUseCaseImpl {
    /// A use case over the engine's operators, derived views and digest.
    pub fn new(
        operators: Arc<dyn OperatorsPort>,
        derived: impl Into<DerivedAccess>,
        digest: Arc<dyn DigestPort>,
    ) -> Self {
        Self {
            operators,
            derived: derived.into(),
            digest,
        }
    }
}

impl PrepareUseCase for PrepareUseCaseImpl {
    fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError> {
        let compiled = match request.plans {
            PlanSource::Authored { plans, bindings } => {
                ryo_execution_domain::domain::prepared_request::service::assemble(
                    plans,
                    bindings,
                    &request.rules,
                )?
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
            Arc::clone(&self.digest),
        )
    }
}
