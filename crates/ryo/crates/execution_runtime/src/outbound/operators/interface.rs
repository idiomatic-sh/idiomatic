//! What the engine asks of the operators capability: one borrowed call in, one pure meaning out; execution authority never crosses it.
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_operators::domain::physical_plan::types::{PlanInputs, PlanNodeId, Resources};
use ryo_operators::inbound::use_cases::operators::types::{OperatorAnswer, OperatorCall};

/// What every `operators` adapter implements.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
/// use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
/// fn planned(port: &dyn OperatorsPort, program: &LogicalProgram<'_>) -> bool {
///     port.plan(program).is_ok()
/// }
/// ```
pub trait OperatorsPort: Send + Sync + std::fmt::Debug {
    /// Runs the operator the call names over its borrowed rows and answers its meaning.
    ///
    /// # Errors
    ///
    /// The portable fault the operator refuses with.
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault>;
    /// The physical plan of a whole program; a node whose kind has no lowering yet is recorded as unserved, never refused.
    ///
    /// # Errors
    ///
    /// The shape fault of a step that is not a valid operation.
    fn plan(&self, program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault>;
    /// Makes `node`'s rows for the open file in `buffers`, running only the pipelines it needs.
    ///
    /// # Errors
    ///
    /// The first fault in the interpreter's order.
    fn run(
        &self,
        plan: &PhysicalPlan,
        node: PlanNodeId,
        buffers: &mut FileBuffers,
        inputs: &dyn PlanInputs,
        resources: &mut dyn Resources,
    ) -> Result<(), PortableOperatorFault>;
}
