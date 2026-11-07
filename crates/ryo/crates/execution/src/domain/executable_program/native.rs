//! Request preparation preserves native tuple geometry, never the old evaluator.
use crate::domain::{
    executable_program::types::{LogicalInstruction, RouteOffers, region::value::Region},
    file_program::types::RequestGraph,
    lowering::types::LoweredShape,
};
use std::sync::Arc;

/// The native shape of each canonical node, taken from the one analysis of its rules.
pub(super) fn capture(graph: &RequestGraph) -> Vec<Option<Arc<LoweredShape>>> {
    graph
        .nodes
        .iter()
        .map(|node| node.analysis.iter().find_map(|step| step.native.clone()))
        .collect()
}
/// Offer the native positions of each region whose single instruction can run over them.
pub(super) fn offer(
    regions: &[Region],
    shapes: &[Option<Arc<LoweredShape>>],
    candidates: &mut [RouteOffers],
) {
    for (region, candidate) in regions.iter().zip(candidates) {
        let [instruction] = region.instructions() else {
            continue;
        };
        if !matches!(
            instruction.operation(),
            ryo_values::domain::plans::types::Operation::Select { .. }
                | ryo_values::domain::plans::types::Operation::Project { .. }
                | ryo_values::domain::plans::types::Operation::Join { .. }
                | ryo_values::domain::plans::types::Operation::Antijoin { .. }
                | ryo_values::domain::plans::types::Operation::Union { .. }
                | ryo_values::domain::plans::types::Operation::Axis { .. }
        ) {
            continue;
        }
        // Zero-column row multiplicity stays on the registered Project route;
        // this alternative declines before performing semantic work.
        if matches!(instruction.operation(), ryo_values::domain::plans::types::Operation::Project { columns, .. } if columns.is_empty())
        {
            continue;
        }
        let LogicalInstruction(logical) = instruction.logical();
        candidate.native = shapes.get(logical).and_then(Clone::clone);
    }
}
