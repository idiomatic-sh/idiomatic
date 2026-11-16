//! The one lowering of a worklist: the columns of its roots, edges and lattice.
use super::types::{WorklistParts, WorklistShape};

/// The one lowering of a worklist: its root, edge and lattice columns.
pub(crate) fn lower(shape: WorklistShape<'_>) -> WorklistParts {
    let [source, destination, order, before, after] = shape.edge_columns;
    let [lower, upper] = shape.lattice_columns;
    WorklistParts {
        root: shape.root,
        state: shape.state,
        source,
        destination,
        order,
        before,
        after,
        lower,
        upper,
        root_keys: shape.root_keys.to_vec(),
        edge_keys: shape.edge_keys.to_vec(),
    }
}
