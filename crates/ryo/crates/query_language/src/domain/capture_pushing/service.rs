//! The `capture_pushing` service: free functions named by verb over the filters made before the capture they follow.

use crate::domain::rewriting::types::Planned;
use crate::domain::rewriting::types::StepIndex;
use crate::domain::walking::service::width_of;
use ryo_values::domain::plan_parts::service::columns_of;
use ryo_values::domain::plan_parts::service::each_column;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::service::each_input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;
use super::types::MovedComparisons;

/// The plan with each filter on the columns a capture was given, made before the capture instead of after it: the capture matches the
/// paths of the rows the filter lets through, not of every row. Not done when another step reads the capture.
#[must_use]
pub(crate) fn push_below_captures(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = push_below_captures_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn push_below_captures_roots(
    mut steps: Vec<PlanStep>,
    roots: &mut [StepId],
) -> Vec<PlanStep> {
    let mut place = 0;
    while place < steps.len() {
        if let Some(MovedComparisons {
            capture,
            input,
            before: moved,
            rest,
        }) = movable(&steps, StepIndex(place), roots)
        {
            let capture = capture.0;
            let before = StepId(u32::try_from(capture).unwrap_or(u32::MAX));
            for step in &mut steps {
                each_input(&mut step.operation, &mut |read| {
                    if let Input::Step(id) = read
                        && id.0 >= before.0
                    {
                        id.0 += 1;
                    }
                });
            }
            for root in roots.iter_mut() {
                if root.0 >= before.0 {
                    root.0 += 1;
                }
            }
            #[expect(
                clippy::indexing_slicing,
                clippy::wildcard_enum_match_arm,
                clippy::unreachable,
                reason = "`movable` found a select at `place` reading a capture at `capture`; the inputs were already renumbered, so any other step here is a bug that must crash, not a wrong plan"
            )]
            {
                match &mut steps[place].operation {
                    Operation::Select { comparisons, .. } => *comparisons = rest,
                    _ => unreachable!(),
                }
                match &mut steps[capture].operation {
                    Operation::Capture { from, .. } => *from = Input::Step(before),
                    _ => unreachable!(),
                }
            }
            #[expect(
                clippy::indexing_slicing,
                reason = "`movable` found a capture at `capture`"
            )]
            let captured = &steps[capture];
            let scope = captured.scope;
            steps.insert(
                capture,
                PlanStep {
                    operation: Operation::Select {
                        from: input,
                        comparisons: moved,
                        kept: Vec::new(),
                    },
                    scope,
                },
            );
            place += 1;
        }
        place += 1;
    }
    steps
}

/// The filter at `place`, when it filters what a capture made on columns the capture was given: the capture, what it reads, and the
/// comparisons that move before it and those that stay.
fn movable(steps: &[PlanStep], place: StepIndex, roots: &[StepId]) -> Option<MovedComparisons> {
    let Operation::Select {
        from: Input::Step(read),
        comparisons,
        ..
    } = &steps.get(place.0)?.operation
    else {
        return None;
    };
    let capture = usize::try_from(read.0).unwrap_or(usize::MAX);
    let captured = steps.get(capture)?;
    let Operation::Capture { from, .. } = &captured.operation else {
        return None;
    };
    // Another step that reads the capture, or an emit of it, would see the rows the filter would take out.
    let readers = steps
        .iter()
        .filter(|step| {
            let mut reads = false;
            let mut copy = step.operation.clone();
            each_input(&mut copy, &mut |each| reads |= *each == Input::Step(*read));
            reads
        })
        .count();
    if readers != 1 || roots.contains(read) || captured.scope != steps.get(place.0)?.scope {
        return None;
    }
    let width = width_of(steps, *from)?.0;
    let (moved, rest): (Vec<PlanComparison>, Vec<PlanComparison>) =
        comparisons.iter().cloned().partition(|comparison| {
            let mut inside = true;
            for operand in [&comparison.left, &comparison.right] {
                each_column(operand, &mut |column| inside &= column.0 < width);
            }
            inside
        });
    // A filter that reads no column is no reason to move anything.
    let reads_a_column = moved.iter().any(|comparison| {
        !columns_of(&comparison.left).is_empty() || !columns_of(&comparison.right).is_empty()
    });
    (reads_a_column).then_some(MovedComparisons {
        capture: StepIndex(capture),
        input: *from,
        before: moved,
        rest,
    })
}
