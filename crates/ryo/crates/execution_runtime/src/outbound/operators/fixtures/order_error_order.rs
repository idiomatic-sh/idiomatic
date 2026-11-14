//! The operators fakes the execution use case's `order_error_order` boundary scenarios inject.
use ryo_values::domain::plans::types::Operation;

/// S13: an operator behind the port that keeps calling its lent driver after a refusal does not replace the first refusal: the
/// run reports the first engine error of the evaluation.
#[derive(Debug)]
pub struct CallsOnAfterARefusal;

impl crate::outbound::operators::interface::OperatorsPort for CallsOnAfterARefusal {
    fn plan(
        &self,
        program: &ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram<'_>,
    ) -> Result<
        ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        ryo_operators::inbound::use_cases::run::fixtures::planned(program)
    }
    fn run(
        &self,
        plan: &ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
        node: ryo_operators::domain::physical_plan::types::PlanNodeId,
        buffers: &mut ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers,
        inputs: &dyn ryo_operators::domain::physical_plan::types::PlanInputs,
        resources: &mut dyn ryo_operators::domain::physical_plan::types::Resources,
    ) -> Result<(), ryo_operators::domain::contract::types::PortableOperatorFault> {
        ryo_operators::inbound::use_cases::run::fixtures::ran(
            plan, node, buffers, inputs, resources,
        )
    }
    fn operators(
        &self,
        mut call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase as _;
        use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
        if let OperatorCall::Evaluate {
            operation: Operation::Select { .. },
            driver,
            ..
        } = &mut call
        {
            // A row-mode evaluation lends no population: refused `NotRun`, the first error.
            let _population =
                driver.population(ryo_values::domain::plan_parts::types::InputNumber(99));
            // A Select is no RenderSyntax: refused `WrongOperation`, the later error.
            let _render = driver.render(&[]);
            return Err(ryo_operators::domain::contract::types::PortableOperatorFault::Interrupted);
        }
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
            std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()),
        )
        .operators(call)
    }
}
