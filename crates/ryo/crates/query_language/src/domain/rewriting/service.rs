//! The `rewriting` service: free functions named by verb over the idiom of a plan that says in many rows what one aggregate says in few: the pairs of functions whose sets of texts are equal.

use super::types::{ComparisonDirection, PairColumns, PairProjection, SameSets};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The operation of the step `id`, when the plan has it.
fn operation_of(steps: &[PlanStep], id: StepId) -> Option<&Operation> {
    steps
        .get(usize::try_from(id.0).unwrap_or(usize::MAX))
        .map(|step| &step.operation)
}

/// The step an input is, when it is one.
fn step_of(input: Input) -> Option<StepId> {
    match input {
        Input::Step(id) => Some(id),
        Input::Relation(_) => None,
    }
}

/// The columns `(left side, left side, right side)` a join keeps, in that order, when it keeps exactly those.
fn three_columns(output: &[JoinColumn]) -> Option<PairColumns> {
    match output {
        [
            JoinColumn {
                side: Side::Left,
                column: first,
            },
            JoinColumn {
                side: Side::Left,
                column: second,
            },
            JoinColumn {
                side: Side::Right,
                column: third,
            },
        ] => Some(PairColumns {
            function: *first,
            other: *second,
            text: *third,
        }),
        _ => None,
    }
}

/// The two columns a projection keeps, when it keeps exactly the first two of its input.
fn first_two(columns: &[ColumnIndex]) -> PairProjection {
    if columns == [ColumnIndex(0), ColumnIndex(1)] {
        PairProjection::FirstTwo
    } else {
        PairProjection::Other
    }
}

/// Reads the projection `mismatches` of an antijoin of joined texts against the texts: which pairs
/// have a text the other side lacks.
fn direction(steps: &[PlanStep], mismatches: StepId) -> Option<ComparisonDirection> {
    let Operation::Project { from, columns, .. } = operation_of(steps, mismatches)? else {
        return None;
    };
    if first_two(columns) == PairProjection::Other {
        return None;
    }
    let Operation::Antijoin {
        left,
        right,
        left_on,
        right_on,
    } = operation_of(steps, step_of(*from)?)?
    else {
        return None;
    };
    let (joined, texts) = (step_of(*left)?, step_of(*right)?);
    let ([first_left, second_left], [owner, text]) = (left_on.as_slice(), right_on.as_slice())
    else {
        return None;
    };
    let Operation::Join {
        left: pairs,
        right: joined_texts,
        left_on: pair_on,
        right_on: owner_on,
        output,
        ..
    } = operation_of(steps, joined)?
    else {
        return None;
    };
    let PairColumns {
        function,
        other,
        text: joined_text,
    } = three_columns(output)?;
    let ([joined_on], [joined_owner]) = (pair_on.as_slice(), owner_on.as_slice()) else {
        return None;
    };
    // The rows joined are (function, other, text) and the antijoin asks for two of them in the texts.
    let asks_text = *second_left == ColumnIndex(2) && joined_text == *text;
    if !asks_text || step_of(*joined_texts)? != texts || *joined_owner != *owner {
        return None;
    }
    Some(ComparisonDirection {
        pairs: step_of(*pairs)?,
        function,
        other,
        texts,
        owner: *owner,
        text: *text,
        joined_on: *joined_on,
        asked: *first_left,
    })
}

/// Whether the antijoin of `left` against `right` on those columns is the idiom for the pairs of
/// `left` whose two functions have equal sets of texts, and what it is made of.
#[must_use]
pub(crate) fn same_sets(
    steps: &[PlanStep],
    left: Input,
    right: Input,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
) -> Option<SameSets> {
    let pairs = step_of(left)?;
    let Operation::Union {
        left: first,
        right: second,
    } = operation_of(steps, step_of(right)?)?
    else {
        return None;
    };
    let (one, two) = (
        direction(steps, step_of(*first)?)?,
        direction(steps, step_of(*second)?)?,
    );
    let ([function, other], [mismatch_function, mismatch_other]) = (left_on, right_on) else {
        return None;
    };
    let both_ways = one.pairs == pairs
        && two.pairs == pairs
        && one.texts == two.texts
        && one.owner == two.owner
        && one.text == two.text;
    // One direction joins the texts of the first function and asks whether the other has them, the second the reverse.
    let first_function = one.joined_on == one.function
        && one.function == *function
        && one.other == *other
        && *mismatch_function == ColumnIndex(0)
        && *mismatch_other == ColumnIndex(1);
    let second_other =
        two.joined_on == two.other && two.function == *function && two.other == *other;
    let asked = one.asked == ColumnIndex(1) && two.asked == ColumnIndex(0);
    (both_ways && first_function && second_other && asked).then_some(SameSets {
        pairs,
        function: *function,
        other: *other,
        texts: one.texts,
        owner: one.owner,
        text: one.text,
    })
}
