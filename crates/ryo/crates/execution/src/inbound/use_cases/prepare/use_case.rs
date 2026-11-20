//! Thin public orchestration over the one private execution owner.
use super::interface::PrepareUseCase;
use super::types::{PlanSource, PrepareRequest};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use std::sync::Arc;

/// Prepares programs over the operators it is given.
#[derive(Debug)]
pub struct PrepareUseCaseImpl {
    operators: Arc<dyn OperatorsPort>,
}

impl PrepareUseCaseImpl {
    /// A use case over the engine's operators.
    pub fn new(operators: Arc<dyn OperatorsPort>) -> Self {
        Self { operators }
    }
}

impl PrepareUseCase for PrepareUseCaseImpl {
    fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError> {
        let mut compiled = match request.plans {
            PlanSource::Authored { plans, bindings } => {
                ryo_execution_domain::domain::prepared_request::service::assemble(
                    plans,
                    bindings,
                    &request.rules,
                )?
            }
            PlanSource::Compiled(queries) => queries.inner,
        };
        // This engine never runs the kinds its `EXPANDED` lists: it is prepared from each plan with those written as the
        // operations it does run. Every caller of this engine passes here; the plans as compiled are what every other
        // reader of a plan keeps, the new engine among them.
        for plan in &mut compiled.plans {
            *plan = Arc::new(ryo_execution_domain::domain::plan_expansion::service::expanded(plan));
        }
        ryo_execution_runtime::application::file_dag::service::prepare(
            compiled,
            &request.rules,
            &request.digests,
            &request.hosted,
            request.observations,
            Arc::clone(&self.operators),
        )
    }
}
