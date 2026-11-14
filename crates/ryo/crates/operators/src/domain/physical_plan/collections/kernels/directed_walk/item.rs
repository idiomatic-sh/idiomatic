//! The `directed_walk` item of the `kernels` collection: from each start row, in order, the entities a walk over the edges
//! reaches in its direction that the domain admits, in walk order; the nearest ancestor the first only. The edges are keyed
//! by parent or child through `hash_index`'s table, the admitted domain entities through its second instance.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::InputRows;
use crate::domain::physical_plan::types::file_buffers::value::{KeyedRows, WalkScratch};
use ryo_values::domain::rows::types::Table;
use crate::domain::axis::types::AxisParts;
use crate::domain::physical_plan::types::KernelRun;

/// `directed_walk`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct DirectedWalkItem;

#[cfg(test)]
impl KernelsItem for DirectedWalkItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = match lowering {
            Lowering::Axis { .. } => true,
            Lowering::Filter { .. }
            | Lowering::Lookup { .. }
            | Lowering::Project { .. }
            | Lowering::Extend { .. }
            | Lowering::Probe { .. }
            | Lowering::Distinct { .. }
            | Lowering::Group { .. }
            | Lowering::Reach { .. }
            | Lowering::SeededReach { .. }
            | Lowering::OrderedWalk { .. }
            | Lowering::Components { .. }
            | Lowering::Unlowered => false,
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// The three inputs of an axis: its start rows, its edge rows and its domain rows.
#[derive(Clone, Copy, Debug)]
#[expect(dead_code, reason = "red step: F3 commit 3 lands the body that reads it")]
pub(crate) struct AxisInputs<'a> {
    /// The start rows.
    pub(crate) starts: InputRows<'a>,
    /// The edge rows.
    pub(crate) edges: InputRows<'a>,
    /// The domain rows.
    pub(crate) domain: InputRows<'a>,
}

/// The two keyed-row tables an axis reads at once: its edges and its admitted domain entities.
#[derive(Debug)]
pub(crate) struct AxisTables<'s> {
    /// The edges, keyed by parent or child.
    pub(crate) edges: &'s mut KeyedRows,
    /// The admitted domain entities.
    pub(crate) domain: &'s mut KeyedRows,
}

/// Writes into `output` each start row's answers, and answers the fault of the first row that raised one.
///
/// # Errors
///
/// A fault of the node's shape: a column past its input's rows.
pub(crate) fn walked(
    axis: &AxisParts,
    inputs: AxisInputs<'_>,
    run: &KernelRun<'_>,
    tables: AxisTables<'_>,
    walk: &mut WalkScratch,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    tables.edges.next.clear();
    tables.domain.next.clear();
    walk.frontier.clear();
    let _ = (axis, inputs, run);
    output.count = ryo_values::domain::rows::types::RowCount(0);
    Ok(None)
}
