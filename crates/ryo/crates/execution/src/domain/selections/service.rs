//! Tree candidates selected for the actual native invocation, with exact visited-row accounting.
use super::evaluation::tree_each_with_visits;
use crate::domain::{
    operating::{errors::OperatingError, types::OperatorContext},
    positions::types::Slots,
    table_cells::types::TreeRows,
};
use ryo_values::domain::{plan_parts::types::Comparison, rows::types::RowIndex};
/// Select tree positions and report each visited candidate before semantic comparison.
/// This keeps logical work accounting exact on early errors and narrowed scans.
pub(crate) fn select_tree_with_visits(
    tree: &TreeRows<'_>,
    comparisons: &[Comparison],
    context: &mut OperatorContext<'_>,
    visit: &mut dyn FnMut(RowIndex),
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let mut buffer = std::mem::take(&mut context.scratch.row);
    let outcome = tree_each_with_visits(
        comparisons,
        tree,
        &mut buffer,
        context,
        visit,
        &mut |index, _, _| {
            out.0.push(u32::try_from(index.0).unwrap_or(u32::MAX));
            Ok(())
        },
    );
    context.scratch.row = buffer;
    outcome
}
