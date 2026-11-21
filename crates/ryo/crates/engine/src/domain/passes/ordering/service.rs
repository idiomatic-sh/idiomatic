//! The service of the `ordering` pass: each level of a unit the statistics measured takes its measured order. The order
//! is decided by `levels::service` from the runs the sample counted at each step's plan point, and no step is written but
//! through the walk that lends a pack (`lowered::service::each_placed_mut`). That walk lends one step at a time, so a
//! level is reordered in two walks: the first takes each of its steps out of its place, the second puts them back in
//! the order the level takes. A step moves with its bodies, so one pair of walks moves the levels of one depth: the
//! programs' own steps first, then the steps of their inline bodies, and so on down, each round read off the pack as the
//! round before left it. A step keeps its plan point wherever it stands.

use super::costs::service::use_costs;
use super::levels::service::{nested_steps, round_at};
use super::types::{OrderRound, RoundEvent};
use crate::domain::lowered::service::each_placed_mut;
use crate::domain::lowered::types::{LentPlaced, LoweredPack};
use crate::domain::statistics::types::Statistics;
use crate::domain::steps::types::{PlacedStep, ProgramStep, RegisterList};
use ryo_relations::domain::relations::types::Count;
use std::collections::VecDeque;

/// What stands in a step's place between the two walks of a round: a step with no body, so the first walk goes into none.
fn hollow() -> PlacedStep {
    PlacedStep {
        point: None,
        step: ProgramStep::Yield {
            row: RegisterList::default(),
        },
    }
}

/// The first walk of a round: every step at the round's depth is taken out of its place, in the walk's order.
fn taken(pack: &mut LoweredPack, round: &OrderRound) -> Vec<PlacedStep> {
    let (mut events, mut steps) = (round.events.iter(), Vec::new());
    each_placed_mut(pack, &mut |_, placed| match placed {
        LentPlaced::Step(step) => match events.next() {
            Some(RoundEvent::Moved) => steps.push(std::mem::replace(step, hollow())),
            Some(RoundEvent::Passed) | None => {}
        },
        LentPlaced::Stage(_) => {}
    });
    steps
}

/// The steps a round took, in the order they go back: each level's steps in the order the level takes.
fn reordered(steps: Vec<PlacedStep>, round: &OrderRound) -> VecDeque<PlacedStep> {
    let mut steps: VecDeque<Option<PlacedStep>> = steps.into_iter().map(Some).collect();
    let mut back = VecDeque::with_capacity(steps.len());
    for level in &round.levels {
        let mut held: Vec<Option<PlacedStep>> =
            steps.drain(..level.len().min(steps.len())).collect();
        for from in level {
            back.extend(held.get_mut(from.0).and_then(Option::take));
        }
    }
    back
}

/// The second walk of a round: every place the first walk emptied takes the next step that goes back. The walk goes into
/// the bodies of the step it was just given, which are below the round's depth: those steps are passed over.
fn given_back(pack: &mut LoweredPack, round: &OrderRound, mut back: VecDeque<PlacedStep>) {
    let (mut events, mut below) = (round.events.iter(), Count(0));
    each_placed_mut(pack, &mut |_, placed| match placed {
        LentPlaced::Step(step) => {
            if below.0 > 0 {
                below = Count(below.0.saturating_sub(1));
                return;
            }
            match (events.next(), back.front()) {
                (Some(RoundEvent::Moved), Some(next)) => {
                    below = nested_steps(&next.step);
                    if let Some(next) = back.pop_front() {
                        *step = next;
                    }
                }
                (Some(RoundEvent::Moved | RoundEvent::Passed) | None, _) => {}
            }
        }
        LentPlaced::Stage(_) => {}
    });
}

/// Each level of a unit the statistics measured takes the measured order; with no statistics, or statistics that measured
/// no unit, every level keeps the lowering's. A unit the statistics do not name, one they measured with other plan points
/// (another lowering of its node) and one whose order they deny keep theirs too. The stages of a table's fill are not
/// reordered: a sample counts no run of a stage.
pub(crate) fn passed(pack: &mut LoweredPack, statistics: Option<&Statistics>) {
    let Some(statistics) = statistics.filter(|statistics| !statistics.units.is_empty()) else {
        return;
    };
    let costs = use_costs(pack, statistics);
    let mut depth = Count(0);
    loop {
        let round = round_at(pack, statistics, &costs, depth);
        if round.levels.is_empty() {
            return;
        }
        // A round in which every level keeps the order it has moves no step: the pack is not walked.
        let kept = |level: &Vec<Count>| level.iter().enumerate().all(|(at, from)| at == from.0);
        if !round.levels.iter().all(kept) {
            let steps = taken(pack, &round);
            let back = reordered(steps, &round);
            given_back(pack, &round, back);
        }
        depth = Count(depth.0.saturating_add(1));
    }
}
