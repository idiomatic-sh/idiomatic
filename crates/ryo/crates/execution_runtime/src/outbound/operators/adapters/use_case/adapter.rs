//! The `operators` port, implemented by the `operators` crate's use cases: the call, the program and the plan are the values the
//! core names, so the adapter passes them through.
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_operators::domain::physical_plan::types::{PlanInputs, PlanNodeId, Resources};
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::types::{OperatorAnswer, OperatorCall};
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase;
use ryo_operators::inbound::use_cases::run::interface::RunUseCase;

/// The `use_case` `operators` adapter.
#[derive(Debug)]
pub(crate) struct OperatorsUseCaseAdapter<U, P, R> {
    /// The `operators` use case of the operators crate, which the edge built.
    operators: U,
    /// The `plan` use case.
    planner: P,
    /// The `run` use case.
    runner: R,
}

impl<U, P, R> OperatorsUseCaseAdapter<U, P, R> {
    /// The adapter over the use cases the edge built.
    pub(crate) const fn new(operators: U, planner: P, runner: R) -> Self {
        Self {
            operators,
            planner,
            runner,
        }
    }
}

impl<U, P, R> OperatorsPort for OperatorsUseCaseAdapter<U, P, R>
where
    U: OperatorsUseCase + Send + Sync + std::fmt::Debug,
    P: PlanUseCase + Send + Sync + std::fmt::Debug,
    R: RunUseCase + Send + Sync + std::fmt::Debug,
{
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        self.operators.operators(call)
    }
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
