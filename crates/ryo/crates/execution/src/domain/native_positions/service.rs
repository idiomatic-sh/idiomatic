//! Pure native operators over borrowed, admitted positions.

use super::{
    axis, joining, selection, sets,
    types::{PositionInput, PositionWork},
};
use crate::domain::{
    lowering::types::LoweredShape,
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::types::{PrimitiveSemantics, WorkCounters},
    positions::types::Positions,
    table_cells::types::TreeRows,
};
use ryo_derived::domain::raw_order::types::raw_order::value::RawOrder;
use ryo_values::domain::plans::types::Operation;

/// Evaluate one already prepared operation. `order` is the raw order the application acquired
/// for the keys of `keys::raw_keys`, borrowed for this call; it is absent when none was needed. Source acquisition, dependency order, publication,
/// and scratch release belong to the application, before and after this call.
pub(crate) fn evaluate(
    semantics: &dyn PrimitiveSemantics,
    order: Option<&RawOrder>,
    operation: &Operation,
    tree: &TreeRows<'_>,
    inputs: &[PositionInput<'_>],
    output: &LoweredShape,
    context: &mut OperatorContext<'_>,
    work: &mut PositionWork,
    counts: &WorkCounters,
) -> Result<Positions, OperatingError> {
    match operation {
        Operation::Select { .. } => {
            selection::select(operation, tree, inputs, output, context, work, counts)
        }
        Operation::Project { .. } => {
            selection::project(order, operation, tree, inputs, output, work, counts)
        }
        Operation::Join { .. } => joining::join(
            semantics, order, operation, tree, inputs, output, context, work, counts,
        ),
        Operation::Antijoin { .. } => sets::antijoin(
            semantics, order, operation, tree, inputs, output, work, counts,
        ),
        Operation::Union { .. } => {
            sets::union(semantics, order, output, work)
        }
        Operation::Axis { .. } => {
            axis::axis(operation, tree, inputs, output, context, work, counts)
        }
        _ => Err(OperatingError::WrongOperation),
    }
}
