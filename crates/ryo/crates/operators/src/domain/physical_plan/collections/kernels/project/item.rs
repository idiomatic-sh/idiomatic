//! The `project` item of the `kernels` collection: a row's columns in order, duplicates kept, through the projection's one
//! meaning; a repeated row dropped by `distinct` when asked.

use super::super::distinct::item::counted;
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use crate::domain::contract::types::PortableOperatorFault;
#[cfg(test)]
use crate::domain::equality_join::types::ProbeMode;
use crate::domain::physical_plan::types::CellStart;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows};
use ryo_values::domain::rows::types::{RowCount, Table};

/// `project`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ProjectItem;

#[cfg(test)]
impl KernelsItem for ProjectItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = {
            match lowering {
                Lowering::Project { .. } => true,
                Lowering::Probe { probe, .. } => probe.mode == ProbeMode::Inner,
                Lowering::Filter { .. }
                | Lowering::Lookup { .. }
                | Lowering::Extend { .. }
                | Lowering::Distinct { .. }
                | Lowering::Group { .. }
                | Lowering::Reach { .. }
                | Lowering::SeededReach { .. }
                | Lowering::Axis { .. }
                | Lowering::OrderedWalk { .. }
                | Lowering::Components { .. }
                | Lowering::Unlowered => false,
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
    seen: &mut hashbrown::HashTable<RowCount>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let start = CellStart(output.cells.len());
    crate::domain::project::service::project(row, columns, &mut output.cells)?;
    counted(output, start, distinct, seen);
    Ok(())
}
