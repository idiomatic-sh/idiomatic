//! Typed semantic requirements bound during request preparation, without engine policy.
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Direction, JoinColumn, Operand, ParameterId, ReachLookup,
};
#[derive(Clone, Debug)]
pub(crate) enum ResourceProgram {
    None,
    Appended(Vec<Operand>),
    Rank(ParameterId),
    Index {
        input: usize,
        columns: IndexColumns,
    },
    Join(JoinResources),
    Count {
        keys: Vec<ColumnIndex>,
        distinct: Option<ColumnIndex>,
    },
    Digest {
        keys: Vec<ColumnIndex>,
        over: ColumnIndex,
    },
    Closure {
        keys: Vec<ColumnIndex>,
        source: ColumnIndex,
        destination: ColumnIndex,
    },
    Cycles {
        source: ColumnIndex,
        destination: ColumnIndex,
    },
    Ordered {
        left_on: Vec<ColumnIndex>,
        right_on: Vec<ColumnIndex>,
        left_order: ColumnIndex,
        right_order: ColumnIndex,
    },
    Dfs(DfsResources),
    Axis(AxisResources),
}
#[derive(Clone, Debug)]
pub(crate) enum IndexColumns {
    Whole,
    Columns(Vec<ColumnIndex>),
    ColumnsUnlessEmpty(Vec<ColumnIndex>),
}
#[derive(Clone, Debug)]
pub(crate) struct JoinResources {
    pub left_on: Vec<ColumnIndex>,
    pub right_on: Vec<ColumnIndex>,
    pub view: Vec<ColumnIndex>,
    pub has_filter: bool,
    pub reach: Option<ReachLookup>,
}
#[derive(Clone, Debug)]
pub(crate) struct AxisResources {
    pub id: ColumnIndex,
    pub parent: ColumnIndex,
    pub child: ColumnIndex,
    pub domain_id: ColumnIndex,
    pub has_filter: bool,
    pub domain_view: Vec<ColumnIndex>,
    pub output: Vec<JoinColumn>,
    pub direction: Direction,
}
#[derive(Clone, Debug)]
pub(crate) struct DfsResources {
    pub root: ColumnIndex,
    pub source: ColumnIndex,
    pub destination: ColumnIndex,
    pub order: ColumnIndex,
    pub edge: ColumnIndex,
    pub root_keys: Vec<ColumnIndex>,
    pub edge_keys: Vec<ColumnIndex>,
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
            input: 1,
            columns: IndexColumns::Whole,
        },
        Operation::Antijoin { right_on, .. } => ResourceProgram::Index {
            input: 1,
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
            has_filter: !filter.is_empty(),
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
            has_filter: !domain_filter.is_empty(),
            domain_view: domain_view.clone(),
            output: output.clone(),
            direction: *direction,
        }),
    }
}

pub(crate) fn binding_dependent(operation: &ryo_values::domain::plans::types::Operation) -> bool {
    use ryo_values::domain::plan_parts::types::Operand;
    fn operand(value: &Operand) -> bool {
        match value {
            Operand::Parameter(_) => true,
            Operand::Function(function) => function.arguments.iter().any(operand),
            Operand::Column(_) | Operand::Literal(_) => false,
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
        | ryo_values::domain::plans::types::Operation::Extend { .. } => return true,
        _ => &[],
    };
    comparisons
        .iter()
        .any(|comparison| operand(&comparison.left) || operand(&comparison.right))
}
