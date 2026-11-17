//! The `project` item of the `kernels` collection: a row's columns in order, duplicates kept, through the projection's one
//! meaning; a repeated row dropped by `distinct` when asked.

use super::super::distinct::item::counted;
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use crate::domain::contract::types::PortableOperatorFault;
#[cfg(test)]
use crate::domain::equality_join::types::ProbeMode;
use crate::domain::growth::types::{Growth, Operator};
use crate::domain::physical_plan::types::CellStart;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::ProbeNode;
use crate::domain::physical_plan::types::lowering::value::ProjectNode;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows};
use ryo_values::domain::rows::types::Table;

/// `project`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ProjectItem;

#[cfg(test)]
impl KernelsItem for ProjectItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = {
            match lowering {
                Lowering::Project(_) => true,
                Lowering::Probe(ProbeNode { probe, .. }) => probe.mode == ProbeMode::Inner,
                Lowering::Filter(_)
                | Lowering::Lookup(_)
                | Lowering::Extend(_)
                | Lowering::Distinct(_)
                | Lowering::Group(_)
                | Lowering::TextMap(_)
                | Lowering::OrderedMatch(_)
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

/// Appends a row's projection to `output`, the first time only when repeated rows are dropped.
///
/// # Errors
///
/// `NoSuchColumn` for a column past the row.
pub(crate) fn projected(
    row: &[Value],
    columns: &[ColumnIndex],
    distinct: RepeatedRows,
    repeats: &mut KeyedRows,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let start = CellStart(output.cells.len());
    crate::domain::project::service::project(row, columns, &mut output.cells)?;
    counted(output, start, distinct, repeats);
    Ok(())
}

impl Operator for ProjectNode {
    fn growth(&self) -> Growth {
        Growth::AtMostInput
    }
}
