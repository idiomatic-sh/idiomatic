//! Answer the original materialization when its complete file sources have already been supplied.
use crate::domain::entity_queries::types::EntityExportId;
use crate::domain::physical_plan::types::{
    PlanNodeId, file_buffers::value::FileBuffers, physical_plan::value::PhysicalPlan,
};
use ryo_values::domain::rows::types::Table;
pub(in super::super) fn answer(
    plan: &PhysicalPlan,
    node: PlanNodeId,
    buffers: &mut FileBuffers,
) -> Option<Table> {
    if crate::spike::entity_off() {
        return None;
    }
    let stage = plan.entity_queries.exported(EntityExportId(node.0))?;
    if buffers.counters.is_some() {
        return None;
    }
    buffers.entity_queries.answer(&plan.entity_queries, stage)
}
