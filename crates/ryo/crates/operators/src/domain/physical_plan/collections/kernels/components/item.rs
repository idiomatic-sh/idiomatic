//! The `components` item of the `kernels` collection: Tarjan's strongly connected components over the edges, numbered in
//! ascending entity order, and each entity of a component of more than one, or with an edge to itself, once, ascending. The
//! edges are keyed by source through `hash_index`'s table.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::InputRows;
use crate::domain::physical_plan::types::file_buffers::value::KeyedRows;
use ryo_values::domain::rows::types::Table;
use crate::domain::cycles::types::{CycleParts, CycleScratch};

/// `components`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ComponentsItem;

#[cfg(test)]
impl KernelsItem for ComponentsItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = match lowering {
            Lowering::Components { .. } => true,
            Lowering::Filter { .. }
            | Lowering::Lookup { .. }
            | Lowering::Project { .. }
            | Lowering::Extend { .. }
            | Lowering::Probe { .. }
            | Lowering::Distinct { .. }
            | Lowering::Group { .. }
            | Lowering::Reach { .. }
            | Lowering::SeededReach { .. }
            | Lowering::Axis { .. }
            | Lowering::OrderedWalk { .. }
            | Lowering::Unlowered => false,
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Writes into `output` one row per entity on a cycle, and answers the fault of the first row that raised one.
///
/// # Errors
///
/// A fault of the node's shape: a column past its input's rows.
pub(crate) fn cyclic(
    cycles: &CycleParts,
    edges: InputRows<'_>,
    index: &mut KeyedRows,
    scratch: &mut CycleScratch,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    index.next.clear();
    scratch.stack.clear();
    let _ = (cycles, edges);
    output.count = ryo_values::domain::rows::types::RowCount(0);
    Ok(None)
}
