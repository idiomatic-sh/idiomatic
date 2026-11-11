//! The types of the `resources` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::derived_views::types::DfsResources;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Direction, JoinColumn, Operand, ParameterId, ReachLookup,
};

/// The side resources an operation acquires before it evaluates, declared by the compiled operation.
#[derive(Clone, Debug)]
pub enum ResourceProgram {
    /// No side resource.
    None,
    /// Operands appended to the input rows.
    Appended(Vec<Operand>),
    /// The literal layers of a rank, by parameter.
    Rank(ParameterId),
    /// A key index of one input.
    Index {
        /// The input indexed.
        input: ryo_values::domain::plan_parts::types::InputNumber,
        /// The columns the index keys on.
        columns: IndexColumns,
    },
    /// The key indexes of a join.
    Join(JoinResources),
    /// Group counts.
    Count {
        /// The columns the groups are keyed on.
        keys: Vec<ColumnIndex>,
        /// The column whose distinct values are counted, when the count is distinct.
        distinct: Option<ColumnIndex>,
    },
    /// A digest of each group.
    Digest {
        /// The columns the groups are keyed on.
        keys: Vec<ColumnIndex>,
        /// The column digested.
        over: ColumnIndex,
    },
    /// The partitioned closure adjacency of a traversal.
    Closure {
        /// The columns the partitions are keyed on.
        keys: Vec<ColumnIndex>,
        /// The column edges start at.
        source: ColumnIndex,
        /// The column edges end at.
        destination: ColumnIndex,
    },
    /// The cycle adjacency of a graph.
    Cycles {
        /// The column edges start at.
        source: ColumnIndex,
        /// The column edges end at.
        destination: ColumnIndex,
    },
    /// The ordered matching of two ordered inputs.
    Ordered {
        /// The left input's key columns.
        left_on: Vec<ColumnIndex>,
        /// The right input's key columns.
        right_on: Vec<ColumnIndex>,
        /// The left input's order column.
        left_order: ColumnIndex,
        /// The right input's order column.
        right_order: ColumnIndex,
    },
    /// The prepared depth-first search.
    Dfs(DfsResources),
    /// The prepared axis walk.
    Axis(AxisResources),
}

/// The columns a key index covers.
#[derive(Clone, Debug)]
pub enum IndexColumns {
    /// Every column of the input.
    Whole,
    /// The listed columns.
    Columns(Vec<ColumnIndex>),
    /// The listed columns, or every column when the list is empty.
    ColumnsUnlessEmpty(Vec<ColumnIndex>),
}

/// Whether a join or an axis carries a filter over its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filtering {
    /// Rows are filtered.
    Filtered,
    /// Rows are not filtered.
    Unfiltered,
}

/// What a join reads: the key columns of each side, the columns it views and its filter.
#[derive(Clone, Debug)]
pub struct JoinResources {
    /// The left input's key columns.
    pub left_on: Vec<ColumnIndex>,
    /// The right input's key columns.
    pub right_on: Vec<ColumnIndex>,
    /// The columns the join reads from its rows.
    pub view: Vec<ColumnIndex>,
    /// Whether the join filters its rows.
    pub has_filter: Filtering,
    /// The reach the join looks up, when it reads one.
    pub reach: Option<ReachLookup>,
}

/// What an axis walk reads: the tree's columns, its domain and the columns it outputs.
#[derive(Clone, Debug)]
pub struct AxisResources {
    /// The column of each node's identity.
    pub id: ColumnIndex,
    /// The column of each node's parent.
    pub parent: ColumnIndex,
    /// The column of each node's child.
    pub child: ColumnIndex,
    /// The column of the domain's node identity.
    pub domain_id: ColumnIndex,
    /// Whether the walk filters its rows.
    pub has_filter: Filtering,
    /// The columns read from the domain.
    pub domain_view: Vec<ColumnIndex>,
    /// The columns the walk outputs.
    pub output: Vec<JoinColumn>,
    /// The direction the walk goes in.
    pub direction: Direction,
}

/// Whether the result of an operation depends on the bindings a request gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingUse {
    Dependent,
    Independent,
}
