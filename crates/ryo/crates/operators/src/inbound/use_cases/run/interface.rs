//! What running a plan offers the engine.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{PlanInputs, PlanNodeId, Resources};

/// Makes one node's rows for the open file, running only the pipelines it needs; its rows are then in `buffers`.
///
/// ```
/// use ryo_operators::inbound::use_cases::run::interface::RunUseCase;
/// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
/// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
/// use ryo_operators::domain::physical_plan::types::{PlanInputs, PlanNodeId, Resources};
/// fn first(run: &impl RunUseCase, plan: &PhysicalPlan, inputs: &dyn PlanInputs, resources: &mut dyn Resources) -> bool {
///     run.run(plan, PlanNodeId(0), &mut FileBuffers::default(), inputs, resources).is_ok()
/// }
/// ```
///
/// Implementations must supply the runner.
///
/// ```compile_fail,E0046
/// use ryo_operators::inbound::use_cases::run::interface::RunUseCase;
/// struct MissingRunner;
/// impl RunUseCase for MissingRunner {}
/// ```
pub trait RunUseCase: Sync {
    /// Makes `node`'s rows for the open file.
    ///
    /// # Errors
    ///
    /// The first fault in the interpreter's order (D15): the earliest input row that fails at any stage, at its first failing
    /// stage; a materialized input's pending fault after the rows before it.
    fn run(
        &self,
        plan: &PhysicalPlan,
        node: PlanNodeId,
        buffers: &mut FileBuffers,
        inputs: &dyn PlanInputs,
        resources: &mut dyn Resources,
    ) -> Result<(), PortableOperatorFault>;
}
