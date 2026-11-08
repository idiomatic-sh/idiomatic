//! Closed typed requirements, construction storage and immutable borrowed access.
use ryo_language::domain::{
    files::types::FilePath,
    symbols::types::QuerySymbols,
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    stored_facts::types::ArenaBytes,
};

use {
    crate::domain::operator::contract::resources::DfsResources,
    crate::domain::stored_graph::types::GraphSource,
    crate::domain::table_cells::types::TreeRows,
    crate::domain::table_texts::types::Texts, ryo_values::domain::files::types::SourceView,
};
use crate::domain::table_model::types::RepresentationInput;

/// The derived view an operation requests, and the columns it is built over.
#[derive(Clone, Debug)]
pub enum ViewRequest {
    /// A hash index of rows by equal keys.
    Equality {
        /// The key columns.
        columns: Vec<ColumnIndex>,
    },
    /// A compact adjacency of edge rows, partitioned by key.
    Adjacency {
        /// The columns of each edge's two endpoints.
        endpoints: [ColumnIndex; 2],
        /// The columns the partitions are keyed on.
        keys: Vec<ColumnIndex>,
    },
    /// The members of one column.
    Membership {
        /// The column whose members are collected.
        column: ColumnIndex,
    },
    /// The rows ordered by a typed key.
    PartitionOrder {
        /// The key columns.
        keys: Vec<ColumnIndex>,
    },
    /// One partition's closure adjacency.
    ClosureAdjacency {
        /// The column edges start at.
        source: ColumnIndex,
        /// The column edges end at.
        destination: ColumnIndex,
    },
    /// The adjacency a cycle search reads.
    CycleAdjacency {
        /// The column edges start at.
        source: ColumnIndex,
        /// The column edges end at.
        destination: ColumnIndex,
    },
    /// The pairing of two ordered inputs by key.
    OrderedKeys {
        /// The left input's key columns.
        left_on: Vec<ColumnIndex>,
        /// The right input's key columns.
        right_on: Vec<ColumnIndex>,
        /// The left input's order column.
        left_order: ColumnIndex,
        /// The right input's order column.
        right_order: ColumnIndex,
    },
    /// The prepared roots and ordered edges of a depth-first search.
    OrderedDfs(DfsResources),
    /// The lookup a static join over native tables reduces to.
    NativeLookup {
        /// The left input's key columns.
        left_on: Vec<ColumnIndex>,
        /// The right input's key columns.
        right_on: Vec<ColumnIndex>,
        /// The columns the join reads from its rows.
        view: Vec<ColumnIndex>,
    },
}
/// The hash of a request's identity, prepared once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IdentityHash(pub u64);

pub mod contract;

/// Membership semantics can address values, existing dense graph vertices or a native tree.
#[derive(Debug)]
pub enum MembershipSpace<'a> {
    /// Any values.
    Values,
    /// The vertices of an existing dense graph.
    Dense(&'a [Value]),
    /// The nodes of a native tree.
    Native(TreeRows<'a>),
}
/// Inputs are semantic capabilities only; none can schedule upstream work or select a cache policy.
#[derive(Debug)]
pub enum DerivedInputs<'a, 's> {
    /// The rows of one partition.
    Partition {
        /// The rows partitioned.
        rows: RepresentationInput<'a>,
        /// The positions of the partition's rows.
        positions: &'a [RowIndex],
        /// The partition's key.
        key: &'a [Value],
    },
    /// Whole inputs, each as one representation.
    Rows(&'a [RepresentationInput<'a>]),
    /// Inputs whose cells are read as text.
    Text {
        /// The inputs.
        rows: &'a [RepresentationInput<'a>],
        /// Where the text of the inputs' names and files is.
        texts: &'a Texts<'a>,
    },
    /// The edge rows of an adjacency.
    Adjacency {
        /// The edge rows.
        rows: RepresentationInput<'a>,
        /// Where the graph's rows come from.
        source: GraphSource,
    },
    /// The rows whose members are collected.
    Membership {
        /// The rows.
        rows: RepresentationInput<'a>,
        /// The space the members are addressed in.
        space: MembershipSpace<'a>,
    },
    /// The roots and edges of a depth-first search, with what it interns derived text into.
    Dfs {
        /// The root rows.
        roots: RepresentationInput<'a>,
        /// The edge rows.
        edges: RepresentationInput<'a>,
        /// The symbols the search interns derived text into.
        symbols: &'a mut QuerySymbols<'s>,
        /// The bytes of the file being read.
        source: SourceView<'a>,
        /// The path of each file of the run, by file number.
        paths: &'a [FilePath],
        /// The text each worker made, by worker number.
        arenas: &'a [ArenaBytes],
    },
}
/// The buffers a derived build reuses between builds.
#[derive(Default, Debug)]
pub struct DerivedScratch {
    /// The derived crate's build workspace.
    pub semantic: ryo_derived::domain::contract::types::BuildWorkspace,
    /// The buffer one row is read into.
    pub row: Vec<Value>,
}
/// How many of a prepared use's input places the view reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputsUsed(pub u8);

use ryo_values::domain::rows::types::RowIndex;

