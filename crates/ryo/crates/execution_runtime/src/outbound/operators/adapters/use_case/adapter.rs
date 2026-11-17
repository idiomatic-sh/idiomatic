//! The `operators` port, implemented by the `operators` crate's use cases: the call, the program and the plan are the values the
//! core names, so the adapter passes them through.
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_operators::domain::physical_plan::types::{PlanInputs, PlanNodeId, Resources};
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase;
use ryo_operators::inbound::use_cases::run::interface::RunUseCase;

/// The `use_case` `operators` adapter.
#[derive(Debug)]
pub(crate) struct OperatorsUseCaseAdapter<P, R> {
    /// The `plan` use case.
    planner: P,
    /// The `run` use case.
    runner: R,
}

impl<P, R> OperatorsUseCaseAdapter<P, R> {
    /// The adapter over the use cases the edge built.
    pub(crate) const fn new(planner: P, runner: R) -> Self {
        Self { planner, runner }
    }
}

impl<P, R> OperatorsPort for OperatorsUseCaseAdapter<P, R>
where
    P: PlanUseCase + Send + Sync + std::fmt::Debug,
    R: RunUseCase + Send + Sync + std::fmt::Debug,
{
    fn plan(&self, program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault> {
        self.planner.plan(program)
    }
    fn run(
        &self,
        plan: &PhysicalPlan,
        node: PlanNodeId,
        buffers: &mut FileBuffers,
        inputs: &dyn PlanInputs,
        resources: &mut dyn Resources,
    ) -> Result<(), PortableOperatorFault> {
        self.runner.run(plan, node, buffers, inputs, resources)
    }
}
