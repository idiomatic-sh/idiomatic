//! The `regrouping` service: free functions named by verb over the per-file inputs of a join after the files, joined with each other
//! in each file before the files hand them over.

use super::types::Regrouped;
use super::types::SplitJoin;
use crate::domain::fusing::service::source_column;
use crate::domain::lineage::service::file_lineage;
use crate::domain::lineage::types::LineageMemo;
use crate::domain::planning::service::planned_scope;
use crate::domain::pushing::service::index_among;
use crate::domain::pushing::types::ColumnPrefixLength;
use crate::domain::rewriting::types::Planned;
use crate::domain::rewriting::types::StepIndex;
use ryo_values::domain::lineage::service::pairing;
use ryo_values::domain::lineage::types::KeyPairing;
use ryo_values::domain::plan_parts::service::column_in;
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::service::each_input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The plan with each join after the files of a per-file input with a join after the files, keyed on columns of that join's per-file
/// input, made two joins: the two per-file inputs joined in each file, and the join's other input joined with that after the files.
/// `Join(Join(C, P1, k1), P2, k2)`, with every column of `k2` a column of `P1` and `k2` pairing a column of each side that states its row's file,
/// becomes `Join(C, Join(P1, P2, k2), k1)`: a file hands over the rows of `P1` and `P2` that meet, not every row of each. Joins are
/// associative, and every pair `k2` matches is of one file, so the rows and their columns are the same. A key of spans alone states
/// no file and leaves the join as it is. Done again on the join it makes, so a chain of such joins is made in
/// each file down to its first input from after the files.
#[must_use]
pub(crate) fn regroup_joins(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = regroup_joins_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn regroup_joins_roots(mut steps: Vec<PlanStep>, roots: &mut [StepId]) -> Vec<PlanStep> {
    let mut place = 0;
    let mut memo = LineageMemo::default();
    while place < steps.len() {
        if let Some(Regrouped { inner, outer }) = regrouped(&steps, &mut memo, StepIndex(place)) {
            // The steps move, so what was found of them no longer holds.
            memo = LineageMemo::default();
            // The join of the per-file inputs takes the place of the join it was made from, which follows it.
            let first = StepId(u32::try_from(place).unwrap_or(u32::MAX));
            for step in &mut steps {
                each_input(&mut step.operation, &mut |input| {
                    if let Input::Step(id) = input
                        && id.0 >= first.0
                    {
                        id.0 += 1;
                    }
                });
            }
            for root in roots.iter_mut() {
                if root.0 >= first.0 {
                    root.0 += 1;
                }
            }
            if let Some(step) = steps.get_mut(place) {
                step.operation = outer;
            }
            steps.insert(
                place,
                PlanStep {
                    operation: inner,
                    scope: StepScope::PerFile,
                },
            );
            place += 1;
        }
        place += 1;
    }
    steps
}

/// The other side.
fn opposite(side: Side) -> Side {
    match side {
        Side::Left => Side::Right,
        Side::Right => Side::Left,
    }
}

/// The join `operation` read as one input from after the files and one per-file input, when it is that, and filters no input.
fn split_join<'a>(steps: &[PlanStep], operation: &'a Operation) -> Option<SplitJoin<'a>> {
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output,
        filter,
        view,
        reach: None,
    } = operation
    else {
        return None;
    };
    if !filter.is_empty() {
        return None;
    }
    match (planned_scope(steps, *left)?, planned_scope(steps, *right)?) {
        (StepScope::CrossFile, StepScope::PerFile) => Some(SplitJoin {
            crossing: *left,
            local: *right,
            crossing_side: Side::Left,
            crossing_keys: left_on,
            local_keys: right_on
                .iter()
                .map(|key| source_column(view, *key))
                .collect::<Option<Vec<_>>>()?,
            local_view: view,
            output,
            view: &[],
        }),
        (StepScope::PerFile, StepScope::CrossFile) => Some(SplitJoin {
            crossing: *right,
            local: *left,
            crossing_side: Side::Right,
            crossing_keys: right_on,
            local_keys: left_on.clone(),
            local_view: &[],
            output,
            view,
        }),
        (StepScope::PerFile, StepScope::PerFile) | (StepScope::CrossFile, StepScope::CrossFile) => {
            None
        }
    }
}

/// The column of the per-file input of `split` that the column `column` of its answer is: its own, or, for a key of the other input,
/// the key of the per-file input it equals.
fn local_column(split: &SplitJoin<'_>, column: ColumnIndex) -> Option<ColumnIndex> {
    let JoinColumn { side, column: at } = *column_in(split.output, column)?;
    if side == split.crossing_side {
        let key = split.crossing_keys.iter().position(|each| *each == at)?;
        split.local_keys.get(key).copied()
    } else {
        source_column(split.local_view, at)
    }
}

/// `column` added to `columns` when it is not there yet.
fn push_new(columns: &mut Vec<ColumnIndex>, column: ColumnIndex) {
    if !columns.contains(&column) {
        columns.push(column);
    }
}

/// The columns an answer keeps of one input, in order.
fn kept_from(side: Side, columns: &[ColumnIndex]) -> impl Iterator<Item = JoinColumn> + '_ {
    columns.iter().map(move |column| JoinColumn {
        side,
        column: *column,
    })
}

/// The two joins the join at `place` is regrouped into, with the join after the files on either side of it.
fn regrouped(steps: &[PlanStep], memo: &mut LineageMemo, place: StepIndex) -> Option<Regrouped> {
    let step = steps.get(place.0)?;
    if step.scope != StepScope::CrossFile {
        return None;
    }
    let at = Input::Step(StepId(u32::try_from(place.0).unwrap_or(u32::MAX)));
    regrouped_on(steps, memo, &step.operation, Side::Left, at)
        .or_else(|| regrouped_on(steps, memo, &step.operation, Side::Right, at))
}

/// The two joins `outer` is regrouped into when the join after the files is its input on `side`; the join of the per-file inputs
/// will be at `at`.
#[expect(
    clippy::too_many_lines,
    reason = "the two sides of both joins are read and rebuilt in one place so their columns are numbered by one rule"
)]
fn regrouped_on(
    steps: &[PlanStep],
    memo: &mut LineageMemo,
    outer: &Operation,
    side: Side,
    at: Input,
) -> Option<Regrouped> {
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output,
        filter,
        view,
        reach: None,
    } = outer
    else {
        return None;
    };
    let (joined, other, joined_keys, other_keys) = match side {
        Side::Left => (*left, *right, left_on, right_on),
        Side::Right => (*right, *left, right_on, left_on),
    };
    // A filter or a view of the right input stays with it: with the other input it moves to the inner join, with the join it cannot.
    if side == Side::Right && !filter.is_empty() {
        return None;
    }
    let Input::Step(joined_id) = joined else {
        return None;
    };
    let first = step_in(steps, joined_id)?;
    if first.scope != StepScope::CrossFile || planned_scope(steps, other)? != StepScope::PerFile {
        return None;
    }
    let split = split_join(steps, &first.operation)?;
    if planned_scope(steps, split.local)? != StepScope::PerFile {
        return None;
    }
    // A column of the first join, as the outer join names it.
    let joined_column = |column: ColumnIndex| match side {
        Side::Left => Some(column),
        Side::Right => source_column(view, column),
    };
    let inner_local_keys = joined_keys
        .iter()
        .map(|key| local_column(&split, joined_column(*key)?))
        .collect::<Option<Vec<_>>>()?;
    let other_source_keys = other_keys
        .iter()
        .map(|key| match side {
            Side::Left => source_column(view, *key),
            Side::Right => Some(*key),
        })
        .collect::<Option<Vec<_>>>()?;
    // Rows of two files could meet on the inner join's keys unless they pair, at one position, a column of each input that states its
    // row's file: a key of spans alone states none, since equal spans of two files are equal.
    let local_lineage = file_lineage(steps, memo, split.local);
    let other_lineage = file_lineage(steps, memo, other);
    if pairing(
        &local_lineage,
        &inner_local_keys,
        &other_lineage,
        &other_source_keys,
        &[],
    ) == KeyPairing::Unproven
    {
        return None;
    }
    let mut local_kept: Vec<ColumnIndex> = Vec::new();
    for key in &split.local_keys {
        push_new(&mut local_kept, *key);
    }
    let mut other_kept: Vec<ColumnIndex> = Vec::new();
    for column in output {
        if column.side == side {
            let made = *column_in(split.output, joined_column(column.column)?)?;
            if made.side != split.crossing_side {
                push_new(
                    &mut local_kept,
                    source_column(split.local_view, made.column)?,
                );
            }
        } else {
            push_new(&mut other_kept, column.column);
        }
    }
    // The inner join keeps the outer join's order of its two inputs, so a filter and a view of a per-file right input stay with it.
    let (inner, local_offset, other_offset) = match side {
        Side::Left => (
            Operation::Join {
                left: split.local,
                right: other,
                left_on: inner_local_keys,
                right_on: other_keys.clone(),
                output: kept_from(Side::Left, &local_kept)
                    .chain(kept_from(Side::Right, &other_kept))
                    .collect(),
                filter: filter.clone(),
                view: view.clone(),
                reach: None,
            },
            ColumnPrefixLength(0),
            ColumnPrefixLength(local_kept.len()),
        ),
        Side::Right => (
            Operation::Join {
                left: other,
                right: split.local,
                left_on: other_keys.clone(),
                right_on: inner_local_keys,
                output: kept_from(Side::Left, &other_kept)
                    .chain(kept_from(Side::Right, &local_kept))
                    .collect(),
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            },
            ColumnPrefixLength(other_kept.len()),
            ColumnPrefixLength(0),
        ),
    };
    let inner_side = opposite(split.crossing_side);
    let inner_keys = split
        .local_keys
        .iter()
        .map(|key| index_among(&local_kept, *key, local_offset))
        .collect::<Option<Vec<_>>>()?;
    let mut answer = Vec::new();
    for column in output {
        answer.push(if column.side == side {
            let made = *column_in(split.output, joined_column(column.column)?)?;
            if made.side == split.crossing_side {
                made
            } else {
                JoinColumn {
                    side: inner_side,
                    column: index_among(
                        &local_kept,
                        source_column(split.local_view, made.column)?,
                        local_offset,
                    )?,
                }
            }
        } else {
            JoinColumn {
                side: inner_side,
                column: index_among(&other_kept, column.column, other_offset)?,
            }
        });
    }
    let regrouped = match split.crossing_side {
        Side::Left => Operation::Join {
            left: split.crossing,
            right: at,
            left_on: split.crossing_keys.to_vec(),
            right_on: inner_keys,
            output: answer,
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        Side::Right => Operation::Join {
            left: at,
            right: split.crossing,
            left_on: inner_keys,
            right_on: split.crossing_keys.to_vec(),
            output: answer,
            filter: Vec::new(),
            view: split.view.to_vec(),
            reach: None,
        },
    };
    Some(Regrouped {
        inner,
        outer: regrouped,
    })
}
