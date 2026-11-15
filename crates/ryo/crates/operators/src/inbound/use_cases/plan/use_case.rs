//! The planner over the physical plan's one dispatch.
use super::interface::PlanUseCase;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;

/// The planner: it holds nothing, so the edge composes it as it is.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlanUseCaseImpl;

impl PlanUseCaseImpl {
    /// The planner, composed once at an edge.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PlanUseCase for PlanUseCaseImpl {
    fn plan(&self, program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault> {
        crate::domain::physical_plan::service::plan(program)
    }
}
