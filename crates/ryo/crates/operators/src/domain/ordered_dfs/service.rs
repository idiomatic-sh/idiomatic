//! Each distinct root is visited once; each edge it reaches is one row, with the ordinal of the visit.
use super::types::{DfsShape, EdgePlace, WalkEdge, WalkEntity};
use crate::domain::contract::types::PortableOperatorFault;

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

/// `ordered-dfs`, one walk: appends to `visits` the place of each edge of `edges` reached from `root`, in visit order.
/// `edges` are one partition's, sorted by source and then by order. The walk expands its root; to expand an entity is to
/// take the edges that leave it, in order; each is a visit, and its destination, when the walk has not met it, is expanded
/// at once, before the source's next edge. An entity is expanded once, so an edge is visited at most once; an edge to an
/// entity the walk has met is visited and not followed. This is what the kernel `ordered_walk` does for one root.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_dfs::service::walked;
/// use ryo_operators::domain::ordered_dfs::types::{EdgePlace, WalkEdge, WalkEntity};
/// use ryo_relations::domain::relations::types::Integer;
/// let edge = |source, destination, order| WalkEdge {
///     source: WalkEntity(source),
///     destination: WalkEntity(destination),
///     order: Integer(order),
/// };
/// // Sorted by source, then order: (1, 2, 1), (1, 3, 2), (2, 3, 0), (3, 1, 0).
/// let edges = [edge(1, 2, 1), edge(1, 3, 2), edge(2, 3, 0), edge(3, 1, 0)];
/// let mut visits = Vec::new();
/// walked(&edges, WalkEntity(1), &mut visits);
/// assert_eq!(visits, [EdgePlace(0), EdgePlace(2), EdgePlace(3), EdgePlace(1)]);
/// ```
pub fn walked(edges: &[WalkEdge], root: WalkEntity, visits: &mut Vec<EdgePlace>) {
    let run = |entity: WalkEntity| {
        let first = edges.partition_point(|edge| edge.source < entity);
        let end = edges.partition_point(|edge| edge.source <= entity);
        (first, end)
    };
    let mut met = std::collections::HashSet::new();
    let _root = met.insert(root);
    let mut frames = vec![run(root)];
    while let Some((next, end)) = frames.last_mut() {
        if *next >= *end {
            let _done = frames.pop();
            continue;
        }
        let at = *next;
        *next = next.saturating_add(1);
        visits.push(EdgePlace(at));
        if let Some(edge) = edges.get(at)
            && met.insert(edge.destination)
        {
            frames.push(run(edge.destination));
        }
    }
}

/// Whether no two edges of one source have one order, in edges sorted by source and then by order: a walk has no order
/// to visit two such edges in.
///
/// # Errors
///
/// `DuplicateOrder` when two edges of one source have one order.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::contract::types::PortableOperatorFault;
/// use ryo_operators::domain::ordered_dfs::service::unrepeated;
/// use ryo_operators::domain::ordered_dfs::types::{WalkEdge, WalkEntity};
/// use ryo_relations::domain::relations::types::Integer;
/// let edge = |destination| WalkEdge { source: WalkEntity(1), destination: WalkEntity(destination), order: Integer(4) };
/// assert_eq!(unrepeated(&[edge(2), edge(3)]), Err(PortableOperatorFault::DuplicateOrder));
/// assert_eq!(unrepeated(&[edge(2)]), Ok(()));
/// ```
pub fn unrepeated(edges: &[WalkEdge]) -> Result<(), PortableOperatorFault> {
    let twice = edges.windows(2).any(|pair| match pair {
        [one, other] => one.source == other.source && one.order == other.order,
        _ => false,
    });
    if twice {
        Err(PortableOperatorFault::DuplicateOrder)
    } else {
        Ok(())
    }
}
