//! The `features` service: free functions over a plan and the rows its steps were fed, with no IO and no ports.

use ryo_relations::domain::relations::service::all;

use super::types::{FeatureRows, StepWork};
use crate::domain::operator::types::RowsFed;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::{Feature, OperationKind};
use ryo_values::domain::records::types::RowTotal;
use ryo_values::domain::run_metadata::types::{FeatureSource, RuleRunMetadata, StepMetadata};

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
fn fed_by_step(step: &StepMetadata, fed: &RowsFed) -> Vec<FeatureRows> {
    let all_inputs: u64 = fed.0.iter().copied().fold(0, u64::saturating_add);
    step.features
        .iter()
        .map(|slot| FeatureRows {
            feature: slot.feature,
            rows: RowTotal(match slot.source {
                FeatureSource::AllInputs => all_inputs,
                FeatureSource::Input(place) => fed.0.get(place.0).copied().unwrap_or(0),
            }),
        })
        .collect()
}

/// How many rows each feature of `plan` was fed, a feature counted once for each step that has it. `fed` holds the rows each
/// step was fed, in the order of the steps.
#[must_use]
pub(crate) fn tally(rule: &RuleRunMetadata, fed: &[RowsFed]) -> Vec<FeatureRows> {
    rule.steps
        .iter()
        .zip(fed)
        .flat_map(|(step, fed)| fed_by_step(step, fed))
        .collect()
}

/// How many rows each step of each plan was fed, summed over its inputs: the work of a rule as a count, the same on any machine at any
/// load. `fed` is the rows each step of each plan was fed, in the order of the plans.
#[must_use]
pub(crate) fn work_over(metadata: &[RuleRunMetadata], fed: &[Vec<RowsFed>]) -> Vec<Vec<StepWork>> {
    metadata
        .iter()
        .zip(fed)
        .map(|(rule, steps)| {
            rule.steps
                .iter()
                .zip(steps)
                .map(|(step, each)| StepWork {
                    operation: step.operation,
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
pub(crate) fn fed_over(metadata: &[RuleRunMetadata], fed: &[Vec<RowsFed>]) -> Vec<FeatureRows> {
    let mut totals: std::collections::BTreeMap<Feature, u64> = std::collections::BTreeMap::new();
    for (rule, steps) in metadata.iter().zip(fed) {
        for each in tally(rule, steps) {
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
