//! The `distinct` item of the `kernels` collection: a row kept the first time only, its repeats found by keying the rows kept
//! in `hash_index`'s table, so no row is copied out.

use super::super::hash_index::item::first_of_row;
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::physical_plan::types::CellStart;
#[cfg(test)]
use crate::domain::physical_plan::types::Lowering;
use crate::domain::physical_plan::types::file_buffers::value::KeyedRows;
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
                | Lowering::Unlowered => false,
            }
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Counts the row `output` holds from cell `start` as one of its rows, unless `repeated` drops it as a repeat of a row it
/// holds, when it is cut off. `repeats` keys the rows of `output` kept so far, in `hash_index`'s table.
pub(crate) fn counted(
    output: &mut Table,
    start: CellStart,
    repeated: RepeatedRows,
    repeats: &mut KeyedRows,
) {
    if repeated == RepeatedRows::Dropped
        && first_of_row(repeats, output, start) == PredicateOutcome::Fails
    {
        output.cells.truncate(start.0);
        return;
    }
    output.count = RowCount(output.count.0 + 1);
}
