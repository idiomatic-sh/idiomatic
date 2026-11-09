//! Typed semantic requirements bound during request preparation, without engine policy.
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

pub(crate) fn prepare(operation: &ryo_values::domain::plans::types::Operation) -> ResourceProgram {
    use ryo_values::domain::plans::types::Operation;
    match operation {
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Union { .. }
        | Operation::Capture { .. }
        | Operation::Words { .. }
        | Operation::Replace { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SpanBounds { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. } => ResourceProgram::None,
        Operation::Extend { values, .. } => ResourceProgram::Appended(values.clone()),
        Operation::Rank { sets, .. } => ResourceProgram::Rank(*sets),
        Operation::Intersection { .. } | Operation::Difference { .. } => ResourceProgram::Index {
            input: ryo_values::domain::plan_parts::types::InputNumber(1),
            columns: IndexColumns::Whole,
        },
        Operation::Antijoin { right_on, .. } => ResourceProgram::Index {
            input: ryo_values::domain::plan_parts::types::InputNumber(1),
            columns: IndexColumns::Columns(right_on.clone()),
        },
        Operation::Join {
            left_on,
            right_on,
            filter,
            view,
            reach,
            ..
        } => ResourceProgram::Join(JoinResources {
            left_on: left_on.clone(),
            right_on: right_on.clone(),
            view: view.clone(),
            has_filter: if filter.is_empty() {
                Filtering::Unfiltered
            } else {
                Filtering::Filtered
            },
            reach: reach.clone(),
        }),
        Operation::GroupCount { keys, distinct, .. } => ResourceProgram::Count {
            keys: keys.clone(),
            distinct: *distinct,
        },
        Operation::GroupDigest { keys, over, .. } => ResourceProgram::Digest {
            keys: keys.clone(),
            over: *over,
        },
        Operation::Closure {
            keys,
            source,
            destination,
            ..
        } => ResourceProgram::Closure {
            keys: keys.clone(),
            source: *source,
            destination: *destination,
        },
        Operation::Cycles {
            source,
            destination,
            ..
        } => ResourceProgram::Cycles {
            source: *source,
            destination: *destination,
        },
        Operation::OrderedMatch {
            left_on,
            right_on,
            left_order,
            right_order,
            ..
        } => ResourceProgram::Ordered {
            left_on: left_on.clone(),
            right_on: right_on.clone(),
            left_order: *left_order,
            right_order: *right_order,
        },
        Operation::OrderedDfs {
            root,
            source,
            destination,
            order,
            edge,
            root_keys,
            edge_keys,
            ..
        } => ResourceProgram::Dfs(DfsResources {
            root: *root,
            source: *source,
            destination: *destination,
            order: *order,
            edge: *edge,
            root_keys: root_keys.clone(),
            edge_keys: edge_keys.clone(),
        }),
        Operation::Axis {
            id,
            parent,
            child,
            domain_id,
            domain_filter,
            domain_view,
            output,
            direction,
            ..
        } => ResourceProgram::Axis(AxisResources {
            id: *id,
            parent: *parent,
            child: *child,
            domain_id: *domain_id,
            has_filter: if domain_filter.is_empty() {
                Filtering::Unfiltered
            } else {
                Filtering::Filtered
            },
            domain_view: domain_view.clone(),
            output: output.clone(),
            direction: *direction,
        }),
    }
}

/// Whether the result of an operation depends on the bindings a request gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingUse {
    Dependent,
    Independent,
}

pub(crate) fn binding_dependent(
    operation: &ryo_values::domain::plans::types::Operation,
) -> BindingUse {
    use ryo_values::domain::plan_parts::types::Operand;
    fn operand(value: &Operand) -> BindingUse {
        match value {
            Operand::Parameter(_) => BindingUse::Dependent,
            Operand::Function(function) => {
                if function
                    .arguments
                    .iter()
                    .any(|argument| operand(argument) == BindingUse::Dependent)
                {
                    BindingUse::Dependent
                } else {
                    BindingUse::Independent
                }
            }
            Operand::Column(_) | Operand::Literal(_) => BindingUse::Independent,
        }
    }
    let comparisons = match operation {
        ryo_values::domain::plans::types::Operation::Select { comparisons, .. } => {
            comparisons.as_slice()
        }
        ryo_values::domain::plans::types::Operation::Join { filter, .. } => filter.as_slice(),
        ryo_values::domain::plans::types::Operation::Axis { domain_filter, .. } => {
            domain_filter.as_slice()
        }
        ryo_values::domain::plans::types::Operation::Rank { .. }
        | ryo_values::domain::plans::types::Operation::Rows { .. }
        | ryo_values::domain::plans::types::Operation::Extend { .. } => {
            return BindingUse::Dependent;
        }
        _ => &[],
    };
    if comparisons.iter().any(|comparison| {
        operand(&comparison.left) == BindingUse::Dependent
            || operand(&comparison.right) == BindingUse::Dependent
    }) {
        BindingUse::Dependent
    } else {
        BindingUse::Independent
    }
}
