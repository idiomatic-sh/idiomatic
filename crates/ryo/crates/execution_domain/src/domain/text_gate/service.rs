//! The `text_gate` service: the columns the comparisons of a select read, with no IO and no ports.

use crate::domain::table_cells::types::ColumnPlaces;
use ryo_operators::domain::table_scratch::types::ComparisonNumbers;
use ryo_values::domain::plan_parts::service::each_column;
use ryo_values::domain::plan_parts::types::PlanComparison;

/// The places of the columns the comparisons numbered `which` read, each once, in order.
pub(crate) fn needed_of_rest(
    comparisons: &[PlanComparison],
    which: &ComparisonNumbers,
    needed: &mut ColumnPlaces,
) {
    let needed = &mut needed.0;
    needed.clear();
    for number in &which.0 {
        let Some(comparison) = comparisons.get(*number) else {
            continue;
        };
        for operand in [&comparison.left, &comparison.right] {
            each_column(operand, &mut |index| {
                needed.push(usize::try_from(index.0).unwrap_or(usize::MAX));
            });
        }
    }
    needed.sort_unstable();
    needed.dedup();
}
