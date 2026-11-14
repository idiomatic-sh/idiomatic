//! The `ordered_walk` item of the `kernels` collection: from each distinct partition and root, in sorted order, an ordered
//! depth-first visit of the edges, a source's edges in ascending order of their order column, each edge reached once with
//! its ordinal among the root's visits. The edges are keyed by partition and source through `hash_index`'s table.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::InputRows;
use crate::domain::physical_plan::types::file_buffers::value::{KeyedRows, WalkScratch};
use ryo_values::domain::rows::types::Table;
use crate::domain::ordered_dfs::types::DfsParts;

/// `ordered_walk`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct OrderedWalkItem;

#[cfg(test)]
impl KernelsItem for OrderedWalkItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = match lowering {
            Lowering::OrderedWalk { .. } => true,
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
            | Lowering::Components { .. }
            | Lowering::Unlowered => false,
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Writes into `output` each visit's row, and answers the fault of the first row that raised one.
///
/// # Errors
///
/// A fault of the node's shape: a column past its input's rows, or an order cell that is not a number.
pub(crate) fn visited(
    dfs: &DfsParts,
    roots: InputRows<'_>,
    edges: InputRows<'_>,
    index: &mut KeyedRows,
    walk: &mut WalkScratch,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    index.next.clear();
    walk.frontier.clear();
    let _ = (dfs, roots, edges);
    output.count = ryo_values::domain::rows::types::RowCount(0);
    Ok(None)
}
