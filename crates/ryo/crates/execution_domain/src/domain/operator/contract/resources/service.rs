//! The `resources` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::types::{
    AxisResources, BindingUse, Filtering, IndexColumns, JoinResources, ResourceProgram,
};
use crate::domain::derived_views::types::DfsResources;

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
        ryo_values::domain::plans::types::Operation::Project { .. }
        | ryo_values::domain::plans::types::Operation::Union { .. }
        | ryo_values::domain::plans::types::Operation::Intersection { .. }
        | ryo_values::domain::plans::types::Operation::Difference { .. }
        | ryo_values::domain::plans::types::Operation::Antijoin { .. }
        | ryo_values::domain::plans::types::Operation::GroupCount { .. }
        | ryo_values::domain::plans::types::Operation::GroupDigest { .. }
        | ryo_values::domain::plans::types::Operation::Capture { .. }
        | ryo_values::domain::plans::types::Operation::Replace { .. }
        | ryo_values::domain::plans::types::Operation::OrderedMatch { .. }
        | ryo_values::domain::plans::types::Operation::Words { .. }
        | ryo_values::domain::plans::types::Operation::Closure { .. }
        | ryo_values::domain::plans::types::Operation::OrderedDfs { .. }
        | ryo_values::domain::plans::types::Operation::Cycles { .. }
        | ryo_values::domain::plans::types::Operation::Hosted { .. }
        | ryo_values::domain::plans::types::Operation::SpanBetween { .. }
        | ryo_values::domain::plans::types::Operation::SourceMatches { .. }
        | ryo_values::domain::plans::types::Operation::RenderSyntax { .. }
        | ryo_values::domain::plans::types::Operation::FitsRendered { .. }
        | ryo_values::domain::plans::types::Operation::SpanBounds { .. } => &[],
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
