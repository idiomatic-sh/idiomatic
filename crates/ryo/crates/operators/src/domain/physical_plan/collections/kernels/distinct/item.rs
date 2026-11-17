//! The `distinct` item of the `kernels` collection: a row kept the first time only, its repeats found by keying the rows kept
//! in `hash_index`'s table, so no row is copied out.

use super::super::hash_index::item::first_of_row;
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::growth::types::{Growth, Operator};
use crate::domain::physical_plan::types::CellStart;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
use crate::domain::physical_plan::types::lowering::value::DistinctNode;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::ProbeNode;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::ProjectNode;
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
                Lowering::Distinct(_) => true,
                Lowering::Project(ProjectNode { project, .. }) => {
                    project.distinct == RepeatedRows::Dropped
                }
                Lowering::Probe(ProbeNode { probe, .. }) => probe.repeated == RepeatedRows::Dropped,
                Lowering::Filter(_)
                | Lowering::Lookup(_)
                | Lowering::Extend(_)
                | Lowering::Group(_)
                | Lowering::TextMap(_)
                | Lowering::OrderedMatch(_)
                | Lowering::BandJoin(_)
                | Lowering::Render(_)
                | Lowering::Reach(_)
                | Lowering::SeededReach(_)
                | Lowering::Axis(_)
                | Lowering::OrderedWalk(_)
                | Lowering::Worklist(_)
                | Lowering::Components(_)
                | Lowering::ScanBound(_) => false,
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

impl Operator for DistinctNode {
    fn growth(&self) -> Growth {
        Growth::AtMostInput
    }
}
