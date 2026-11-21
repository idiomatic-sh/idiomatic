//! The `costing` service's steps part: what one step costs and gives the steps after it, read as the sweep of a query's steps
//! reaches it.

use ryo_relations::domain::relations::types::{Relation, RelationSize};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::Operation;

use super::super::super::types::{
    AssembledSteps, CostShape, CostedStep, Filtering, JoinFacts, JoinKeys, PlanCostClass,
    RangeBound, Refining, SelfOrder, ShapeClass, SideRows, WrittenSite,
};
use super::super::joins::service::{both, file_alone, join_verdict, refining, worse};
use crate::domain::distinctness::service::distinct_on;
use crate::domain::distinctness::types::DistinctMemo;
use crate::domain::lowering::types::Uniqueness;

/// Reads the `where` of a join's statement into the join's verdict: `operation` when it filters the rows of a join that the
/// statement `written` also wrote. Each alternative of a `where` with `or` is one filter of the join, and the join's verdict is
/// the greatest of theirs.
pub(crate) fn refine_join(
    assembled: AssembledSteps<'_>,
    sizes: fn(Relation) -> Option<RelationSize>,
    kept: &mut [CostedStep],
    operation: &Operation,
    written: Option<&WrittenSite>,
) {
    let Operation::Select {
        from: Input::Step(read),
        comparisons,
        ..
    } = operation
    else {
        return;
    };
    let place = usize::from(*read);
    let (Some(joined), Some(join)) = (assembled.steps.get(place), kept.get_mut(place)) else {
        return;
    };
    let (
        Some(facts),
        Operation::Join {
            left,
            right,
            output,
            ..
        },
    ) = (join.join, &joined.operation)
    else {
        return;
    };
    if assembled.written.get(place).map(Option::as_ref) != Some(written) {
        return;
    }
    let refined = worse(
        join_verdict(
            facts,
            refining(assembled, [*left, *right], output, comparisons),
        ),
        pair_scan(assembled, sizes, &joined.operation),
    );
    join.verdict = match join.filtering {
        Filtering::Bare => refined,
        Filtering::Filtered => worse(join.verdict, refined),
    };
    join.filtering = Filtering::Filtered;
}

/// What one step costs: its shape and class when it is above linear, and for a join what its keys say, for the `where` that
/// may follow it in its statement. An operation the language gains gets its arm here: no arm stands for the others.
///
/// A join one side of which holds each value of the join's keys once gives no more rows than its other side has: on the file
/// alone, that is a side of one row a file, and the join is linear. The proof is the `distinctness` concept's, which reads the
/// catalog and the operations alone and never a rule or a relation by name: a relation's key column, the file of a relation the
/// catalog declares one row a file, a count or a digest of groups asked of every column that keys them, a projection that
/// drops repeated rows asked of every column it keeps, and what a filter, a projection, an extension, an antijoin, a group's
/// end or a join to one row at most keeps of those.
pub(crate) fn step_cost(
    assembled: AssembledSteps<'_>,
    sizes: fn(Relation) -> Option<RelationSize>,
    distinct: &mut DistinctMemo,
    operation: &Operation,
) -> CostedStep {
    let mut cost = CostedStep {
        verdict: None,
        join: None,
        filtering: Filtering::Bare,
    };
    match operation {
        Operation::Join {
            left,
            right,
            left_on,
            right_on,
            ..
        } => {
            let facts = JoinFacts {
                keys: both(
                    file_alone(assembled, *left, left_on),
                    file_alone(assembled, *right, right_on),
                ),
                side: match (
                    distinct_on(assembled.steps, distinct, *left, left_on),
                    distinct_on(assembled.steps, distinct, *right, right_on),
                ) {
                    (Uniqueness::Shared, Uniqueness::Shared) => SideRows::Unproved,
                    (Uniqueness::Unique, Uniqueness::Unique | Uniqueness::Shared)
                    | (Uniqueness::Shared, Uniqueness::Unique) => SideRows::Bounded,
                },
            };
            cost.join = Some(facts);
            cost.verdict = join_verdict(
                facts,
                Refining {
                    keys: JoinKeys::FileAlone,
                    range: RangeBound::Open,
                    order: SelfOrder::Unordered,
                },
            );
        }
        Operation::AsOf { .. }
        | Operation::OrderedMatch { .. }
        | Operation::GroupBound { .. }
        | Operation::GroupOrdinal { .. }
        | Operation::OrderedDfs { .. } => {
            cost.verdict = Some(ShapeClass {
                shape: CostShape::Sort,
                class: PlanCostClass::LinearWithSorts,
            });
        }
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Antijoin { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::Follow { .. }
        | Operation::Worklist { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => {}
    }
    cost.verdict = worse(cost.verdict, pair_scan(assembled, sizes, operation));
    cost
}

/// The verdict of a step that scans a relation the catalog declares a row for a pair of elements: every input of the operation
/// that is such a relation, but the side of a join, an antijoin, an as-of or an ordered match that is keyed beyond the file.
fn pair_scan(
    assembled: AssembledSteps<'_>,
    sizes: fn(Relation) -> Option<RelationSize>,
    operation: &Operation,
) -> Option<ShapeClass> {
    let keyed = |input: Input, on: &[ColumnIndex]| file_alone(assembled, input, on);
    let sides = match operation {
        Operation::Join {
            left,
            right,
            left_on,
            right_on,
            ..
        }
        | Operation::Antijoin {
            left,
            right,
            left_on,
            right_on,
        }
        | Operation::AsOf {
            left,
            right,
            left_on,
            right_on,
            ..
        }
        | Operation::OrderedMatch {
            left,
            right,
            left_on,
            right_on,
            ..
        } => [keyed(*left, left_on), keyed(*right, right_on)],
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::GroupBound { .. }
        | Operation::GroupOrdinal { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::Follow { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Worklist { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => [JoinKeys::FileAlone, JoinKeys::FileAlone],
    };
    named(&inputs_of(operation))
        .iter()
        .zip(sides.iter().chain(std::iter::repeat(&JoinKeys::FileAlone)))
        .any(|(input, keys)| {
            matches!(input, Input::Relation(relation)
                if sizes(*relation) == Some(RelationSize::PerPair) && *keys == JoinKeys::FileAlone)
        })
        .then_some(ShapeClass {
            shape: CostShape::OrderedPairs,
            class: PlanCostClass::QuadraticInGroup,
        })
}
