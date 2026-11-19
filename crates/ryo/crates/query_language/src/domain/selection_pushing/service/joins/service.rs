//! The `selection_pushing` service's join part: a filter of a join placed on the join's inputs, and an equality of a column of
//! each input made a key of the join.

use super::super::super::types::CrossingKey;
use super::super::super::types::Keyed;
use super::super::super::types::PushedFilter;
use super::super::super::types::SplitComparisons;
use super::super::placing::service::{filter_of, finished, met_key, place_after};
use crate::domain::lineage::service::column_kind;
use crate::domain::lineage::types::LineageMemo;
use crate::domain::rewriting::types::StepIndex;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::service::column_in;
use ryo_values::domain::plan_parts::service::fixed_prefix;
use ryo_values::domain::plan_parts::service::remapped_comparison;
use ryo_values::domain::plan_parts::service::rides_the_walk;
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::RowWidth;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The comparisons of a filter of the join of `left` on `left_on` with `right` on `right_on`, answering `output` through
/// `view`, by where they are decided.
fn split_comparisons(
    steps: &[PlanStep],
    comparisons: &[PlanComparison],
    join: Keyed<'_>,
) -> SplitComparisons {
    let Keyed {
        left_on,
        right_on,
        output,
        view,
        ..
    } = join;
    let side_column = |side: Side| {
        move |column: ColumnIndex| {
            let made = column_in(output, column)?;
            if made.side != side {
                return None;
            }
            match side {
                Side::Left => Some(made.column),
                Side::Right => view_source(view, made.column),
            }
        }
    };
    // A key column of one input as the key it meets in the other, the right's read through the view.
    let read_on: Vec<ColumnIndex> = right_on
        .iter()
        .map(|key| view_source(view, *key))
        .collect::<Option<_>>()
        .unwrap_or_default();
    let right_key = |column: ColumnIndex| met_key(left_on, &read_on, column);
    let left_key = |column: ColumnIndex| met_key(&read_on, left_on, column);
    let mut memo = LineageMemo::default();
    let prefix = fixed_prefix(comparisons, RowWidth(output.len())).0;
    let mut split = SplitComparisons::default();
    for comparison in prefix {
        if let Some(left) = remapped_comparison(comparison, &side_column(Side::Left)) {
            split.right.extend(remapped_comparison(&left, &right_key));
            split.left.push(left);
        } else if let Some(right) = remapped_comparison(comparison, &side_column(Side::Right)) {
            split.left.extend(remapped_comparison(&right, &left_key));
            split.right.push(right);
        } else if let Some(key) = crossing_key(steps, &mut memo, comparison, join) {
            split.left_on.push(key.left);
            split.right_on.push(key.right);
        } else {
            split.rest.push(comparison.clone());
        }
    }
    split.rest.extend(
        comparisons
            .get(prefix.len()..)
            .unwrap_or_default()
            .iter()
            .cloned(),
    );
    split
}

/// The left column and the right column, as the view places it, of an equality of a column of each input of the join, when the
/// two hold cells of one kind a join's key compares as the filter's `=` does: a node, a number or a flag. A filter compares a
/// name or a file by its text and a span by its start, which is not how a join's key compares them, so such an equality stays.
/// The kind of one of the two is enough: the compiler compares a node, a number or a flag column only with a column of its own
/// kind (`compatible`), and a column the plan cannot follow (a closure's, say) is of the kind of the column it is compared with.
fn crossing_key(
    steps: &[PlanStep],
    memo: &mut LineageMemo,
    comparison: &PlanComparison,
    join: Keyed<'_>,
) -> Option<CrossingKey> {
    let (Test::Equal, Operand::Column(first), Operand::Column(second)) =
        (comparison.test, &comparison.left, &comparison.right)
    else {
        return None;
    };
    let (first, second) = (
        *column_in(join.output, *first)?,
        *column_in(join.output, *second)?,
    );
    let (left, right) = match (first.side, second.side) {
        (Side::Left, Side::Right) => (first.column, second.column),
        (Side::Right, Side::Left) => (second.column, first.column),
        (Side::Left, Side::Left) | (Side::Right, Side::Right) => return None,
    };
    let left_kind = column_kind(steps, memo, join.left, left);
    let right_kind = column_kind(steps, memo, join.right, view_source(join.view, right)?);
    if left_kind
        .zip(right_kind)
        .is_some_and(|(one, other)| one != other)
    {
        return None;
    }
    matches!(
        left_kind.or(right_kind)?,
        CellKind::Node | CellKind::Number | CellKind::Flag
    )
    .then_some(CrossingKey { left, right })
}

/// What takes the place of the filter at `reader` of the join `id`, when some of its comparisons are placed on an input or
/// become keys of the join.
pub(crate) fn pushed_join(
    steps: &[PlanStep],
    id: StepId,
    reader: StepIndex,
) -> Option<PushedFilter> {
    let made = step_in(steps, id)?;
    let filtering = steps.get(reader.0)?;
    let Operation::Select {
        comparisons, kept, ..
    } = &filtering.operation
    else {
        return None;
    };
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output,
        filter,
        view,
        reach: None,
    } = &made.operation
    else {
        return None;
    };
    let split = split_comparisons(
        steps,
        comparisons,
        Keyed {
            left: *left,
            right: *right,
            left_on,
            right_on,
            output,
            view,
        },
    );
    if split.left.is_empty() && split.right.is_empty() && split.left_on.is_empty() {
        return None;
    }
    let left_on: Vec<ColumnIndex> = left_on.iter().chain(&split.left_on).copied().collect();
    let right_on: Vec<ColumnIndex> = right_on.iter().chain(&split.right_on).copied().collect();
    let first = StepId(u32::try_from(reader.0).unwrap_or(u32::MAX));
    let mut before = Vec::new();
    let mut from = |input: Input, moved: Vec<PlanComparison>| -> Option<Input> {
        if moved.is_empty() {
            return Some(input);
        }
        let at = place_after(first, &before);
        before.push(filter_of(steps, input, moved)?);
        Some(at)
    };
    let placed_left = from(*left, split.left)?;
    // The right comparisons ride the probe where it walks the tree by a node; elsewhere they are a filter of the right, so the
    // probe's index is the filter's node and is shared as one.
    let walked: Vec<PlanComparison> = filter.iter().chain(&split.right).cloned().collect();
    let rides = !split.right.is_empty()
        && matches!(right, Input::Relation(relation)
            if rides_the_walk(*relation, &right_on, view, &walked).is_some());
    let (placed_right, placed_filter) = if rides {
        (*right, walked)
    } else {
        (from(*right, split.right)?, filter.clone())
    };
    // A filter that only keeps columns, once its comparisons are placed, is the join's answer itself.
    let (answer, kept) = if split.rest.is_empty() && !kept.is_empty() {
        (
            kept.iter()
                .map(|column| column_in(output, *column).copied())
                .collect::<Option<Vec<_>>>()?,
            Vec::new(),
        )
    } else {
        (output.clone(), kept.clone())
    };
    let combined = PlanStep {
        operation: Operation::Join {
            left: placed_left,
            right: placed_right,
            left_on,
            right_on,
            output: answer,
            filter: placed_filter,
            view: view.clone(),
            reach: None,
        },
        scope: made.scope,
    };
    Some(finished(
        reader, before, combined, filtering, split.rest, kept,
    ))
}
