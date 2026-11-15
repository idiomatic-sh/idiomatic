//! Each distinct root is visited once; each edge it reaches is one row, with the ordinal of the visit.
use super::types::DfsShape;

/// The one lowering of an ordered depth-first visit: its root and edge columns.
pub(crate) fn lower(shape: DfsShape<'_>) -> super::types::DfsParts {
    let [source, destination, order, edge] = shape.edge_columns;
    super::types::DfsParts {
        root: shape.root,
        source,
        destination,
        order,
        edge,
        root_keys: shape.root_keys.to_vec(),
        edge_keys: shape.edge_keys.to_vec(),
    }
}
