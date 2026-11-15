//! What an ordered depth-first search reads from its operation.
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The root column, the edge columns each row carries, and the partition keys of each side.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DfsShape<'a> {
    /// The root column of a root row.
    pub(crate) root: ColumnIndex,
    /// The source, destination, order and edge columns of an edge row, in that order.
    pub(crate) edge_columns: [ColumnIndex; 4],
    /// The partition key columns of a root row.
    pub(crate) root_keys: &'a [ColumnIndex],
    /// The partition key columns of an edge row, matched to the root's in order.
    pub(crate) edge_keys: &'a [ColumnIndex],
}

/// What an ordered depth-first visit lowers to: the root and edge columns, and the partition each side keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DfsParts {
    /// The root column of a root row.
    pub(crate) root: ColumnIndex,
    /// The source column of an edge row.
    pub(crate) source: ColumnIndex,
    /// The destination column of an edge row.
    pub(crate) destination: ColumnIndex,
    /// The numeric order column of an edge row: a source's edges are visited in its ascending order.
    pub(crate) order: ColumnIndex,
    /// The edge column of an edge row.
    pub(crate) edge: ColumnIndex,
    /// The partition key columns of a root row.
    pub(crate) root_keys: Vec<ColumnIndex>,
    /// The partition key columns of an edge row.
    pub(crate) edge_keys: Vec<ColumnIndex>,
}

/// One edge an ordered depth-first visit reached, with its ordinal among the edges visited from its root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderedEdge {
    /// The edge row, by its place in the prepared edges.
    pub edge: ryo_values::domain::rows::types::RowIndex,
    /// The ordinal of the visit.
    pub ordinal: ryo_relations::domain::relations::types::Integer,
}

/// The engine's prepared ordered depth-first graph and its walk.
pub trait DfsDriver {
    /// How many root rows the graph holds, in their prepared sorted order.
    ///
    /// # Errors
    ///
    /// The engine retained an error acquiring the graph.
    fn roots(
        &mut self,
    ) -> Result<
        ryo_values::domain::rows::types::RowCount,
        crate::domain::contract::types::PortableOperatorFault,
    >;
    /// Copies the root row at a place of the sorted order into `row`.
    fn root(
        &self,
        at: ryo_values::domain::rows::types::RowIndex,
        row: &mut Vec<ryo_relations::domain::relations::types::Value>,
    ) -> crate::domain::contract::types::RowRead;
    /// Copies the root columns of the prepared graph into `columns`: the partition keys followed by the
    /// canonical root.
    fn root_columns(&self, columns: &mut Vec<ryo_values::domain::plan_parts::types::ColumnIndex>);
    /// Starts a visit from a root row's partition and canonical root.
    ///
    /// # Errors
    ///
    /// The engine retained an error starting the walk.
    fn begin(
        &mut self,
        partition: &[ryo_relations::domain::relations::types::Value],
        root: ryo_relations::domain::relations::types::Value,
    ) -> Result<(), crate::domain::contract::types::PortableOperatorFault>;
    /// The next edge of the visit, or none when it is over.
    ///
    /// # Errors
    ///
    /// The engine retained an error walking the graph.
    fn next(
        &mut self,
        partition: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<Option<OrderedEdge>, crate::domain::contract::types::PortableOperatorFault>;
    /// Copies an edge row into `row`.
    fn edge(
        &self,
        at: ryo_values::domain::rows::types::RowIndex,
        row: &mut Vec<ryo_relations::domain::relations::types::Value>,
    ) -> crate::domain::contract::types::RowRead;
}

/// An engine driver that lends its ordered depth-first graph and walk.
pub trait DfsEvaluation: crate::domain::contract::types::RowDriver {
    /// The engine's ordered depth-first graph and walk.
    fn dfs(&mut self) -> &mut dyn DfsDriver;
}
