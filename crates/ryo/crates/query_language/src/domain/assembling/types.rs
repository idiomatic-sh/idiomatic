//! The types of the `assembling` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::costing::types::PlanCost;
use ryo_values::domain::query_plans::types::Plan;

/// A program lowered: the plan the engine walks, and what the query costs as its text states it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Lowered {
    /// The plan.
    pub plan: Plan,
    /// The cost, read off the steps as assembled, before the plan's rewrites.
    pub cost: PlanCost,
}
