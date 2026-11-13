//! The types of the `adjacency` concept: its values are declared in their own modules.
use crate::domain::allocation::types::ElementCapacity;

pub mod compact_graph;

pub mod forest_parents;

/// The capacity of each buffer of a compact graph its accessors do not lend, and how many values its partitions hold.
///
/// # Examples
///
/// ```
/// use ryo_derived::domain::adjacency::types::GraphCapacities;
/// use ryo_derived::domain::allocation::types::ElementCapacity;
/// let none = ElementCapacity(0);
/// let empty = GraphCapacities {
///     partitions: none,
///     partition_count: none,
///     partition_values: none,
///     partition_ids: none,
///     vertices: none,
///     vertex_ids: none,
///     forests: [None, None],
/// };
/// assert_eq!(empty.partition_values, none);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphCapacities {
    /// The capacity of the partition list.
    pub partitions: ElementCapacity,
    /// How many partitions the graph holds, each its own allocation.
    pub partition_count: ElementCapacity,
    /// How many values the partitions hold together.
    pub partition_values: ElementCapacity,
    /// The capacity of the partition lookup.
    pub partition_ids: ElementCapacity,
    /// The capacity of the vertex list.
    pub vertices: ElementCapacity,
    /// The capacity of the vertex lookup.
    pub vertex_ids: ElementCapacity,
    /// The capacity of each direction's forest parents, when it has a forest.
    pub forests: [Option<ElementCapacity>; 2],
}
