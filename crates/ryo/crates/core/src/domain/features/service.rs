//! The `features` service: free functions over a plan and the rows its steps were fed, with no IO and no ports.

use ryo_language::domain::relations::service::all;

use super::types::{FeatureRows, StepWork};
use crate::domain::operator::types::RowsFed;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Feature, Operation, OperationKind, Plan, PlanStep};
use ryo_values::domain::records::types::RowTotal;

/// The kind of an operation.
#[must_use]
pub(crate) fn kind_of(operation: &Operation) -> OperationKind {
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
fn comparisons_of(operation: &Operation) -> &[Comparison] {
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

/// Every feature the language has: each operation, each way of comparing, each direction of an axis, each relation.
#[must_use]
pub(crate) fn catalog() -> Vec<Feature> {
    [
        OperationKind::Select,
        OperationKind::Project,
        OperationKind::Extend,
        OperationKind::Join,
        OperationKind::Union,
        OperationKind::Intersection,
        OperationKind::Difference,
        OperationKind::Antijoin,
        OperationKind::Replace,
        OperationKind::SpanBetween,
        OperationKind::SourceMatches,
        OperationKind::RenderSyntax,
        OperationKind::FitsRendered,
        OperationKind::SpanBounds,
        OperationKind::OrderedMatch,
        OperationKind::OrderedDfs,
        OperationKind::GroupCount,
        OperationKind::GroupDigest,
        OperationKind::Capture,
        OperationKind::Words,
        OperationKind::Rank,
        OperationKind::Axis,
        OperationKind::Closure,
        OperationKind::Cycles,
        OperationKind::Rows,
        OperationKind::Hosted,
    ]
    .into_iter()
    .map(Feature::Operation)
    .chain(
        [
            Test::Equal,
            Test::NotEqual,
            Test::LessThan,
            Test::LessThanOrEqual,
            Test::GreaterThan,
            Test::GreaterThanOrEqual,
            Test::FloatTextGreaterThanOrEqual,
            Test::NaturalLessThan,
            Test::NumericGreaterThanOrEqual,
            Test::IntegerSuccessor,
            Test::In,
            Test::NotIn,
            Test::GlobAny,
            Test::NotGlobAny,
            Test::StartsWith,
            Test::EndsWith,
            Test::Matches,
        ]
        .into_iter()
        .map(Feature::Family),
    )
    .chain(
        [
            Direction::Parent,
            Direction::Child,
            Direction::Ancestor,
            Direction::Descendant,
            Direction::NearestAncestor,
            Direction::Deep,
            Direction::PreviousSibling,
            Direction::NextSibling,
        ]
        .into_iter()
        .map(Feature::Direction),
    )
    .chain(all().iter().copied().map(Feature::Relation))
    .collect()
}

/// How many rows each feature of `step` was fed, given the rows each of its inputs was: an operation, a way of comparing and a
/// direction take every input, a relation the input that reads it.
fn fed_by_step(step: &PlanStep, fed: &RowsFed) -> Vec<FeatureRows> {
    let all_inputs: u64 = fed.0.iter().copied().fold(0, u64::saturating_add);
    let mut features = vec![FeatureRows {
        feature: Feature::Operation(kind_of(&step.operation)),
        rows: RowTotal(all_inputs),
    }];
    for comparison in comparisons_of(&step.operation) {
        features.push(FeatureRows {
            feature: Feature::Family(comparison.test),
            rows: RowTotal(all_inputs),
        });
    }
    if let Operation::Axis { direction, .. } = &step.operation {
        features.push(FeatureRows {
            feature: Feature::Direction(*direction),
            rows: RowTotal(all_inputs),
        });
    }
    let inputs = inputs_of(&step.operation);
    for (place, input) in named(&inputs).iter().enumerate() {
        if let Input::Relation(relation) = input {
            features.push(FeatureRows {
                feature: Feature::Relation(*relation),
                rows: RowTotal(fed.0.get(place).copied().unwrap_or(0)),
            });
        }
    }
    features
}

/// How many rows each feature of `plan` was fed, a feature counted once for each step that has it. `fed` holds the rows each
/// step was fed, in the order of the steps.
#[must_use]
pub(crate) fn tally(plan: &Plan, fed: &[RowsFed]) -> Vec<FeatureRows> {
    plan.steps
        .iter()
        .zip(fed)
        .flat_map(|(step, fed)| fed_by_step(step, fed))
        .collect()
}

/// How many rows each step of each plan was fed, summed over its inputs: the work of a rule as a count, the same on any machine at any
/// load. `fed` is the rows each step of each plan was fed, in the order of the plans.
#[must_use]
pub(crate) fn work_over(plans: &[Plan], fed: &[Vec<RowsFed>]) -> Vec<Vec<StepWork>> {
    plans
        .iter()
        .zip(fed)
        .map(|(plan, steps)| {
            plan.steps
                .iter()
                .zip(steps)
                .map(|(step, each)| StepWork {
                    operation: kind_of(&step.operation),
                    rows: RowTotal(
                        each.0
                            .iter()
                            .fold(0_u64, |sum, rows| sum.saturating_add(*rows)),
                    ),
                })
                .collect()
        })
        .collect()
}

/// How many rows each feature of the language was fed over a run, in the catalog's order: the rows of the steps of every plan that
/// has it, summed. `fed` is the rows each step of each plan was fed, in the order of the plans.
#[must_use]
pub(crate) fn fed_over(plans: &[Plan], fed: &[Vec<RowsFed>]) -> Vec<FeatureRows> {
    let mut totals: std::collections::BTreeMap<Feature, u64> = std::collections::BTreeMap::new();
    for (plan, steps) in plans.iter().zip(fed) {
        for each in tally(plan, steps) {
            let slot = totals.entry(each.feature).or_default();
            *slot = slot.saturating_add(each.rows.0);
        }
    }
    catalog()
        .into_iter()
        .map(|feature| FeatureRows {
            feature,
            rows: RowTotal(totals.get(&feature).copied().unwrap_or(0)),
        })
        .collect()
}
