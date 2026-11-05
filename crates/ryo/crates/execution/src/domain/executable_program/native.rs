//! Request preparation preserves native tuple geometry, never the old evaluator.
use crate::domain::{
    executable_program::types::{ExecutableProgram, LogicalInstruction},
    file_program::types::RequestGraph,
    lowering::types::{Counting, LoweredShape},
};
use std::sync::Arc;

/// The physical analysis is transient. Retain only immutable output coordinates.
pub(super) fn capture(graph: &RequestGraph) -> Vec<Option<Arc<LoweredShape>>> {
    graph
        .nodes
        .iter()
        .map(|node| {
            if node.phase != ryo_values::domain::plans::types::StepScope::PerFile {
                return None;
            }
            node.regions.iter().find_map(|region| {
                let index = region.step.0 as usize;
                if region.layout.lowering.counting.get(index) != Some(&Counting::DoesNotCount) {
                    return None;
                }
                let shape = region.layout.lowering.shapes.get(index)?.as_ref()?;
                Some(Arc::new(shape.clone()))
            })
        })
        .collect()
}
pub(super) fn install(program: &mut ExecutableProgram, shapes: Vec<Option<Arc<LoweredShape>>>) {
    for region in &mut program.regions {
        let [instruction] = region.instructions.as_ref() else {
            continue;
        };
        if !matches!(
            instruction.operation,
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
        if matches!(&instruction.operation, ryo_values::domain::plans::types::Operation::Project { columns, .. } if columns.is_empty())
        {
            continue;
        }
        let LogicalInstruction(logical) = instruction.logical;
        region.native_positions = shapes.get(logical).and_then(Clone::clone);
    }
}
