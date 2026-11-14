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
/// use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
/// let vocabulary = KindVocabulary { language: LanguageName("none".into()), kinds: Vec::new() };
/// let plan = PlanUseCaseImpl.plan(&LogicalProgram::new(&vocabulary));
/// assert!(plan.is_ok_and(|plan| plan.unserved().is_empty()));
/// ```
///
/// A kernel is private to operators: the engine cannot name one.
///
/// ```compile_fail,E0603
/// use ryo_operators::domain::physical_plan::service::plan;
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
    /// The physical plan of `program`. A node whose kind has no lowering yet is recorded in `PhysicalPlan::unserved`, never
    /// answered as an error, so one unserved rule never fails the served ones.
    ///
    /// # Errors
    ///
    /// The shape fault of a step that is not a valid operation: `WrongOperation` for a node that reads a later node.
    fn plan(&self, program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault>;
}
