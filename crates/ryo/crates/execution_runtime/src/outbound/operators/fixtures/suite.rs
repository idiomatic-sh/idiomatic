//! The operators fakes the execution use case's `suite` boundary scenarios inject.

#[derive(Debug, Default)]
pub struct ReductionAudit {
    pub reductions: std::sync::atomic::AtomicUsize,
}

impl crate::outbound::operators::interface::OperatorsPort for ReductionAudit {
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
        use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
        use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
        use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
        if let OperatorCall::ReduceCount { .. } = &mut call {
            self.reductions
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        OperatorsUseCase::operators(
            &OperatorsUseCaseImpl::new(std::sync::Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            )),
            call,
        )
    }
}
