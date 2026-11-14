//! The operators-port witness and refusal the capability proofs share with the join proofs beside them.
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use std::sync::atomic::{AtomicUsize, Ordering};
pub(crate) const REFUSAL: u32 = 60001;
#[derive(Debug, Default)]
pub(crate) struct Witness {
    pub(crate) calls: AtomicUsize,
    pub(crate) refuse: bool,
}
impl crate::outbound::operators::interface::OperatorsPort for Witness {
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
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if let OperatorCall::Evaluate {
            operation: ryo_values::domain::plans::types::Operation::Join { .. },
            ..
        } = &call
        {
            let _before = self.calls.fetch_add(1, Ordering::Relaxed);
            if self.refuse {
                return Err(PortableOperatorFault::NoSuchColumn {
                    index: ryo_values::domain::plan_parts::types::ColumnIndex(REFUSAL),
                });
            }
        }
        crate::application::file_dag::fixtures::operators().operators(call)
    }
}
