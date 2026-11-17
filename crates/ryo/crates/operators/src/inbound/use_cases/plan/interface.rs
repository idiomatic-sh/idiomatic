//! What planning a program offers the engine.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;

/// The physical plan of a whole program: each kind's one lowering, the rewrites its consumers' demand allows, pipelines broken
/// only at blocking kernels and shared nodes.
///
/// ```
/// use ryo_operators::inbound::use_cases::plan::{interface::PlanUseCase, use_case::PlanUseCaseImpl};
/// use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
/// let plan = PlanUseCaseImpl.plan(&LogicalProgram::new());
/// assert!(plan.is_ok());
/// ```
///
/// A kernel is private to operators: the engine cannot name one.
///
/// ```compile_fail,E0603
/// use ryo_operators::domain::physical_plan::service::plan;
/// ```
///
/// Nor its kernels, nor what a node is lowered to:
///
/// ```compile_fail,E0603
/// use ryo_operators::domain::physical_plan::collections::kernels::reach::item::reached;
/// ```
///
/// ```compile_fail,E0603
/// use ryo_operators::domain::physical_plan::types::lowering::value::Lowering;
/// ```
///
/// Implementations must supply the planner.
///
/// ```compile_fail,E0046
/// use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase;
/// struct MissingPlanner;
/// impl PlanUseCase for MissingPlanner {}
/// ```
pub trait PlanUseCase: Sync {
    /// The physical plan of `program`: every node lowered once.
    ///
    /// # Errors
    ///
    /// The shape fault of a step that is not a valid operation: `WrongOperation` for a node that reads a later node.
    fn plan(&self, program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault>;
}
