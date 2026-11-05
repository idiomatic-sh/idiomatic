//! The `plans` service: free functions named by verb over a compiled plan, with no IO and no ports.

use super::types::{Operation, OperationKind, Plan, StepScope};
use crate::domain::plan_parts::service::step_in;
use crate::domain::plan_parts::types::Comparison;
use crate::domain::plan_parts::types::Input;
use crate::domain::plan_parts::types::StepId;
use crate::domain::plan_parts::types::{InputCount, Inputs};

/// What an operation reads, in the order its operator reads its inputs.
#[must_use]
pub fn inputs_of(operation: &Operation) -> Inputs {
    let (first, second, third, used) = match operation {
        Operation::SpanBetween { from, .. }
        | Operation::SpanBounds { from, .. }
        | Operation::SourceMatches { from, .. }
        | Operation::RenderSyntax { from, .. }
        | Operation::FitsRendered { from, .. }
        | Operation::Replace { from, .. }
        | Operation::Select { from, .. }
        | Operation::Project { from, .. }
        | Operation::Extend { from, .. }
        | Operation::GroupCount { from, .. }
        | Operation::GroupDigest { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Rank { from, .. }
        | Operation::Words { from, .. } => (*from, *from, *from, 1),
        Operation::OrderedMatch { left, right, .. }
        | Operation::Join { left, right, .. }
        | Operation::Union { left, right }
        | Operation::Intersection { left, right }
        | Operation::Difference { left, right }
        | Operation::Antijoin { left, right, .. } => (*left, *right, *left, 2),
        Operation::OrderedDfs { from, edges, .. } => (*from, *edges, *from, 2),
        Operation::Axis {
            from,
            edges,
            domain,
            ..
        } => (*from, *edges, *domain, 3),
        Operation::Closure { edges, .. } | Operation::Cycles { edges, .. } => {
            (*edges, *edges, *edges, 1)
        }
        // The host's rows read nothing: the place filled in is never asked for.
        Operation::Rows { .. } | Operation::Hosted { .. } => {
            let nothing = Input::Step(StepId(0));
            (nothing, nothing, nothing, 0)
        }
    };
    Inputs {
        list: [first, second, third],
        used: InputCount(used),
    }
}

/// The inputs an operation names, without the places that repeat.
#[must_use]
pub fn named(inputs: &Inputs) -> &[Input] {
    inputs.list.get(..inputs.used.0).unwrap_or(&inputs.list)
}

/// Where the step `id` runs, when the plan has it.
#[must_use]
pub fn step_scope(plan: &Plan, id: StepId) -> Option<StepScope> {
    step_in(&plan.steps, id).map(|step| step.scope)
}

/// Gives `visit` each input the operation reads.
pub fn each_input(operation: &mut Operation, visit: &mut dyn FnMut(&mut Input)) {
    match operation {
        Operation::SpanBetween { from, .. }
        | Operation::SpanBounds { from, .. }
        | Operation::SourceMatches { from, .. }
        | Operation::RenderSyntax { from, .. }
        | Operation::FitsRendered { from, .. }
        | Operation::Replace { from, .. }
        | Operation::Select { from, .. }
        | Operation::Project { from, .. }
        | Operation::Extend { from, .. }
        | Operation::GroupCount { from, .. }
        | Operation::GroupDigest { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Rank { from, .. }
        | Operation::Words { from, .. } => visit(from),
        Operation::OrderedMatch { left, right, .. }
        | Operation::Join { left, right, .. }
        | Operation::Union { left, right }
        | Operation::Intersection { left, right }
        | Operation::Difference { left, right }
        | Operation::Antijoin { left, right, .. } => {
            visit(left);
            visit(right);
        }
        Operation::OrderedDfs { from, edges, .. } => {
            visit(from);
            visit(edges);
        }
        Operation::Axis {
            from,
            edges,
            domain,
            ..
        } => {
            visit(from);
            visit(edges);
            visit(domain);
        }
        Operation::Closure { edges, .. } | Operation::Cycles { edges, .. } => visit(edges),
        Operation::Rows { .. } | Operation::Hosted { .. } => {}
    }
}

/// The kind of an operation.
#[must_use]
pub fn kind_of(operation: &Operation) -> OperationKind {
    match operation {
        Operation::SpanBetween { .. } => OperationKind::SpanBetween,
        Operation::SourceMatches { .. } => OperationKind::SourceMatches,
        Operation::RenderSyntax { .. } => OperationKind::RenderSyntax,
        Operation::FitsRendered { .. } => OperationKind::FitsRendered,
        Operation::SpanBounds { .. } => OperationKind::SpanBounds,
        Operation::Select { .. } => OperationKind::Select,
        Operation::Project { .. } => OperationKind::Project,
        Operation::Extend { .. } => OperationKind::Extend,
        Operation::Join { .. } => OperationKind::Join,
        Operation::Union { .. } => OperationKind::Union,
        Operation::Intersection { .. } => OperationKind::Intersection,
        Operation::Difference { .. } => OperationKind::Difference,
        Operation::Antijoin { .. } => OperationKind::Antijoin,
        Operation::Replace { .. } => OperationKind::Replace,
        Operation::OrderedMatch { .. } => OperationKind::OrderedMatch,
        Operation::OrderedDfs { .. } => OperationKind::OrderedDfs,
        Operation::GroupCount { .. } => OperationKind::GroupCount,
        Operation::GroupDigest { .. } => OperationKind::GroupDigest,
        Operation::Capture { .. } => OperationKind::Capture,
        Operation::Words { .. } => OperationKind::Words,
        Operation::Rank { .. } => OperationKind::Rank,
        Operation::Axis { .. } => OperationKind::Axis,
        Operation::Closure { .. } => OperationKind::Closure,
        Operation::Cycles { .. } => OperationKind::Cycles,
        Operation::Rows { .. } => OperationKind::Rows,
        Operation::Hosted { .. } => OperationKind::Hosted,
    }
}

/// The comparisons an operation makes.
pub fn comparisons_of(operation: &Operation) -> &[Comparison] {
    match operation {
        Operation::Select { comparisons, .. } => comparisons,
        Operation::Join { filter, .. } => filter,
        Operation::Axis { domain_filter, .. } => domain_filter,
        Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::Replace { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. }
        | Operation::OrderedMatch { .. }
        | Operation::OrderedDfs { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => &[],
    }
}
