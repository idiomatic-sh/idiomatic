//! What the planner made of a logical program: the kernels of each lowered node and the nodes it could not lower yet.

use crate::domain::physical_plan::types::Unserved;

/// The physical plan of one program. Built only by `plan`, so the engine cannot write a kernel.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
/// fn unlowered(plan: &PhysicalPlan) -> usize {
///     plan.unserved().len()
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct PhysicalPlan {
    /// The nodes whose kind has no lowering yet, in node order.
    unserved: Vec<Unserved>,
}

impl PhysicalPlan {
    /// The nodes whose kind has no lowering yet, in node order, each with the stage that lowers it. `plan` records them and
    /// never fails for them; `assemble` turns them into per-rule refusals. Branch only: deleted in F5.
    #[must_use]
    pub fn unserved(&self) -> &[Unserved] {
        &self.unserved
    }
}
