//! The `distinct` item of the `kernels` collection: a row kept the first time only, its repeats found in the row set of the
//! rows kept, so no row is copied out.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use crate::domain::physical_plan::types::CellStart;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::row_set::value::{FirstSeen, RowSet};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::rows::types::{RowCount, Table};

/// `distinct`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct DistinctItem;

#[cfg(test)]
impl KernelsItem for DistinctItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = {
            match lowering {
                Lowering::Distinct { .. } => true,
                Lowering::Project { project, .. } => project.distinct == RepeatedRows::Dropped,
                Lowering::Probe { probe, .. } => probe.repeated == RepeatedRows::Dropped,
                Lowering::Filter { .. }
                | Lowering::Lookup { .. }
                | Lowering::Extend { .. }
                | Lowering::Group { .. }
                | Lowering::TextMap { .. }
                | Lowering::OrderedMatch { .. }
                | Lowering::Render { .. }
                | Lowering::Reach { .. }
                | Lowering::SeededReach { .. }
                | Lowering::Axis { .. }
                | Lowering::OrderedWalk { .. }
                | Lowering::Worklist { .. }
                | Lowering::Components { .. }
                | Lowering::ScanBound { .. }
                | Lowering::Unlowered => false,
            }
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Counts the row `output` holds from cell `start` as one of its rows, unless `repeated` drops it as a repeat of a row it
/// holds, when it is cut off. `repeats` keeps the rows of `output` kept so far, once each.
pub(crate) fn counted(
    output: &mut Table,
    start: CellStart,
    repeated: RepeatedRows,
    repeats: &mut RowSet,
) {
    if repeated == RepeatedRows::Dropped && repeats.first_seen(output, start) == FirstSeen::Repeat {
        output.cells.truncate(start.0);
        return;
    }
    output.count = RowCount(output.count.0 + 1);
}
