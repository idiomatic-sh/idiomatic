//! The service of the `numbering` pass: every step and stage of a unit takes its plan point, its place in the unit's
//! pre-order. A unit is what is counted as one program: a definition's pattern in call form; a rule's own program, all its
//! branches; a table's fill, all its pipelines. The order is the shared walk's (`lowered::service::each_touched`), taken
//! through the walk that lends a pack to write in that one order (`lowered::service::each_placed_mut`): a step, then the
//! steps of its inline bodies in its operands' order; a fill's pipelines in its inputs' order.

use crate::domain::lowered::service::each_placed_mut;
use crate::domain::lowered::types::{LentPlaced, LoweredPack, WalkedUnit};
use crate::domain::steps::types::PlanPoint;

/// Takes the next point of a unit: `next` is the point the next step or stage takes, none once the unit holds more than a
/// point can number.
fn taken(next: &mut Option<PlanPoint>) -> Option<PlanPoint> {
    let point = *next;
    *next = point
        .and_then(|taken| taken.0.checked_add(1))
        .map(PlanPoint);
    point
}

/// Every step and stage takes its plan point: its place in its unit's pre-order, from nought in each unit. A point is the
/// step's own from here on: a pass that moves a placed step moves its point with it, and a step a later pass adds has none.
pub(crate) fn passed(pack: &mut LoweredPack) {
    let mut numbering: Option<(WalkedUnit, Option<PlanPoint>)> = None;
    each_placed_mut(pack, &mut |unit, placed| {
        // A unit's steps and stages are met one after the other: its points start from nought when the unit changes.
        if numbering.map(|(of, _)| of) != Some(unit) {
            numbering = Some((unit, Some(PlanPoint(0))));
        }
        let Some((_, next)) = numbering.as_mut() else {
            return;
        };
        match placed {
            LentPlaced::Step(step) => step.point = taken(next),
            LentPlaced::Stage(stage) => stage.point = taken(next),
        }
    });
}
