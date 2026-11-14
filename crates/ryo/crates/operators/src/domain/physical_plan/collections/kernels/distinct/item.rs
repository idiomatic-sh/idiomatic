//! The `distinct` item of the `kernels` collection: a row kept the first time only, its repeats found through the places of
//! the rows kept, so no row is copied out.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use crate::domain::physical_plan::types::CellStart;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
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
                | Lowering::Unlowered => false,
            }
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Counts the row `output` holds from cell `start` as one of its rows, unless `repeated` drops it as a repeat of a row it
/// holds, when it is cut off. `seen` holds the place of each row of `output` kept so far.
pub(crate) fn counted(
    output: &mut Table,
    start: CellStart,
    repeated: RepeatedRows,
    seen: &mut hashbrown::HashTable<RowCount>,
) {
    if repeated == RepeatedRows::Dropped {
        let width = output.kinds.len();
        let hasher = rustc_hash::FxBuildHasher;
        let cells = &output.cells;
        let row_at = |at: RowCount| {
            cells
                .get(at.0 * width..at.0 * width + width)
                .unwrap_or_default()
        };
        let new = cells.get(start.0..).unwrap_or_default();
        let hash = std::hash::BuildHasher::hash_one(&hasher, new);
        if seen.find(hash, |earlier| row_at(*earlier) == new).is_some() {
            output.cells.truncate(start.0);
            return;
        }
        let _: hashbrown::hash_table::OccupiedEntry<'_, RowCount> =
            seen.insert_unique(hash, output.count, |earlier| {
                std::hash::BuildHasher::hash_one(&hasher, row_at(*earlier))
            });
    }
    output.count = RowCount(output.count.0 + 1);
}
