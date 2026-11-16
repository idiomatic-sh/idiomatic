//! What a worklist reads from its operation, and the work its kernel keeps across nodes and files.
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

/// The columns a `worklist` operation names, as the planner reads them from it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WorklistShape<'a> {
    /// The root column of a root row.
    pub(crate) root: ColumnIndex,
    /// The starting state column of a root row.
    pub(crate) state: ColumnIndex,
    /// The source, destination, order, before-state and after-state columns of an edge row, in that order.
    pub(crate) edge_columns: [ColumnIndex; 5],
    /// The lower and upper state columns of a lattice row, in that order.
    pub(crate) lattice_columns: [ColumnIndex; 2],
    /// The partition key columns of a root row.
    pub(crate) root_keys: &'a [ColumnIndex],
    /// The partition key columns of an edge row, matched to the root's in order.
    pub(crate) edge_keys: &'a [ColumnIndex],
}

/// What a worklist lowers to: the columns of its three inputs, and the partition each side keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorklistParts {
    /// The root column of a root row.
    pub(crate) root: ColumnIndex,
    /// The starting state column of a root row.
    pub(crate) state: ColumnIndex,
    /// The source column of an edge row.
    pub(crate) source: ColumnIndex,
    /// The destination column of an edge row.
    pub(crate) destination: ColumnIndex,
    /// The numeric order column of an edge row: a source's edges are followed in its ascending order.
    pub(crate) order: ColumnIndex,
    /// The state an edge's source must hold for the edge to be followed.
    pub(crate) before: ColumnIndex,
    /// The state an edge joins into its destination.
    pub(crate) after: ColumnIndex,
    /// The lower state column of a lattice row.
    pub(crate) lower: ColumnIndex,
    /// The upper state column of a lattice row.
    pub(crate) upper: ColumnIndex,
    /// The partition key columns of a root row.
    pub(crate) root_keys: Vec<ColumnIndex>,
    /// The partition key columns of an edge row.
    pub(crate) edge_keys: Vec<ColumnIndex>,
}

/// A state of the declared lattice, by its place among the lattice's states in ascending cell order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct StatePlace(pub(crate) u32);

/// The state an entity of the open run holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HeldState {
    /// The entity, as its canonical cell.
    pub(crate) entity: Value,
    /// The state it holds.
    pub(crate) state: StatePlace,
}

/// Whether one state of a lattice is at or below another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StateOrder {
    /// It is at or below it.
    AtOrBelow,
    /// It is not.
    Unordered,
}

/// Which state of a lattice row a cell is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LatticeSide {
    /// The lower state.
    Lower,
    /// The upper state.
    Upper,
}

/// The place of a pair of states in a lattice's order table: the first's place times the states, plus the second's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OrderPlace(pub(crate) usize);

/// A worklist's declared lattice, tabled before any run: its states, the order between them, the least upper bound of every
/// two, and the least state; its capacity kept across nodes and files.
#[derive(Debug, Default)]
pub(crate) struct LatticeTable {
    /// The states, ascending, each once, as canonical cells.
    pub(crate) states: Vec<Value>,
    /// The cell each state is written as: the first lattice cell of its text, in `states`' order.
    pub(crate) named: Vec<Value>,
    /// Whether one state is at or below another, row-major by the first state's place.
    pub(crate) below: Vec<StateOrder>,
    /// The least upper bound of each pair of states, row-major by the first state's place.
    pub(crate) joins: Vec<StatePlace>,
    /// The least state, which every entity holds before any join.
    pub(crate) least: StatePlace,
}

/// The work of a worklist, its capacity kept across nodes and files.
#[derive(Debug, Default)]
pub(crate) struct WorklistScratch {
    /// The lattice, tabled.
    pub(crate) lattice: LatticeTable,
    /// The state each entity of the open run holds; an entity it holds none for holds the least state.
    pub(crate) held: hashbrown::HashTable<HeldState>,
    /// The entities waiting to leave the queue, first in first out, an entity once for each change of its state.
    pub(crate) queue: std::collections::VecDeque<Value>,
    /// The edge rows, sorted by partition, source, before-state and order.
    pub(crate) edges: Vec<RowIndex>,
    /// The root rows, sorted by partition and root.
    pub(crate) roots: Vec<RowIndex>,
    /// The canonical cells of the lattice, then of the edges: each edge's partition, source, destination, before-state and
    /// after-state, row by row.
    pub(crate) cells: Vec<Value>,
    /// The canonical cells of the roots: each root's partition, root and state, row by row.
    pub(crate) root_cells: Vec<Value>,
    /// The texts no symbol of the file holds, numbered after them.
    pub(crate) fresh: Vec<Value>,
}
