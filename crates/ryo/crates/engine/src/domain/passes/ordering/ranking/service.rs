//! The service of the `ranking` part: the order of the steps of one level. Of the steps whose register dependencies are
//! placed, the one with the lowest rank goes next. A step's rank is (rate - 1) / work: a test that passes few rows for
//! little work goes early, an iteration that gives many goes late. A step is ranked by the best run of steps it unlocks
//! (a read is followed through to the test it feeds), so a cheap read that makes a selective test possible is not held
//! back by its own rank of nought. A step never moves before a step it depends on: one that writes a register it reads
//! or writes, or reads a register it writes.

use crate::domain::passes::ordering::constants::{LEAST_GAIN, LEAST_WORK, LOOKAHEAD};
use crate::domain::passes::ordering::types::{LevelStep, PassRate, RunWork, StepOrder, StepRank};
use crate::domain::steps::types::Register;
use ryo_relations::domain::relations::types::Count;

/// Whether `later` must stay after `earlier`: a flow, anti or output dependence on a register.
fn order_of(earlier: &LevelStep, later: &LevelStep) -> StepOrder {
    let overlap = |left: &[Register], right: &[Register]| {
        left.iter().any(|register| right.contains(register))
    };
    if overlap(&earlier.writes, &later.reads)
        || overlap(&earlier.reads, &later.writes)
        || overlap(&earlier.writes, &later.writes)
    {
        StepOrder::Held
    } else {
        StepOrder::Free
    }
}

/// The rank of a rate reached for a work: the lower, the earlier.
fn rank(rate: PassRate, work: RunWork) -> StepRank {
    StepRank((rate.0 - 1.0) / work.0.max(LEAST_WORK.0))
}

/// The modelled work of an order for each entry of the level: each step's work times the rates of the steps before it.
#[must_use]
pub(crate) fn modelled(order: &[Count], steps: &[LevelStep]) -> RunWork {
    let (mut reach, mut total) = (1.0, 0.0);
    for step in order.iter().filter_map(|place| steps.get(place.0)) {
        total += reach * step.work.0;
        reach *= step.rate.0;
    }
    RunWork(total)
}

/// The steps each step must stay after, by place.
fn held_after(steps: &[LevelStep]) -> Vec<Vec<Count>> {
    let each = steps.iter().enumerate().map(|(later, step)| {
        let earlier = steps.iter().take(later).enumerate();
        earlier
            .filter(|(_, other)| order_of(other, step) == StepOrder::Held)
            .map(|(place, _)| Count(place))
            .collect()
    });
    each.collect()
}

/// The steps a step must stay after.
fn needs(after: &[Vec<Count>], place: Count) -> &[Count] {
    after.get(place.0).map_or(&[][..], Vec::as_slice)
}

/// The best rank of the run of steps that placing `first` unlocks: the run grows by its lowest-ranked unlocked step, and
/// the answer is the lowest rank any prefix of it reaches.
fn unlocked_rank(
    first: Count,
    steps: &[LevelStep],
    after: &[Vec<Count>],
    placed: &[Count],
) -> Option<StepRank> {
    let start = steps.get(first.0)?;
    let (mut work, mut rate) = (start.work.0, start.rate.0);
    let mut chain = vec![first];
    let mut best = rank(PassRate(rate), RunWork(work));
    for _ in 0..LOOKAHEAD.0 {
        let places = (0..steps.len()).map(Count);
        // The steps this run unlocks: every step it must stay after is placed or in the run, and one is in the run.
        let ready = places.filter(|place| {
            let met = |need: &Count| placed.contains(need) || chain.contains(need);
            !placed.contains(place)
                && !chain.contains(place)
                && needs(after, *place).iter().all(met)
                && needs(after, *place).iter().any(|need| chain.contains(need))
        });
        let next = ready
            .filter_map(|place| Some((place, steps.get(place.0)?)))
            .min_by(|left, right| {
                let (left, right) = (
                    rank(left.1.rate, left.1.work),
                    rank(right.1.rate, right.1.work),
                );
                left.0.total_cmp(&right.0)
            });
        let Some((place, step)) = next else {
            break;
        };
        work += rate * step.work.0;
        rate *= step.rate.0;
        chain.push(place);
        let reached = rank(PassRate(rate), RunWork(work));
        if reached.0 < best.0 {
            best = reached;
        }
    }
    Some(best)
}

/// The order of a level's steps by rank under their dependencies: for each place of the new order, the place its step
/// holds now. Of two steps of one rank the one that stands first now goes first, so the order of a level is one order.
#[must_use]
pub(crate) fn ranked(steps: &[LevelStep]) -> Vec<Count> {
    let after = held_after(steps);
    let mut placed: Vec<Count> = Vec::with_capacity(steps.len());
    while placed.len() < steps.len() {
        let free = (0..steps.len()).map(Count).filter(|place| {
            !placed.contains(place)
                && needs(&after, *place)
                    .iter()
                    .all(|need| placed.contains(need))
        });
        let ranks =
            free.filter_map(|place| Some((place, unlocked_rank(place, steps, &after, &placed)?)));
        // The first of the lowest: `min_by` answers the first of equal ranks.
        let Some((next, _)) = ranks.min_by(|left, right| left.1.0.total_cmp(&right.1.0)) else {
            // A step depends only on steps before it, so one is always free: nothing is left out of an order.
            return (0..steps.len()).map(Count).collect();
        };
        placed.push(next);
    }
    placed
}

/// The order a level takes: its ranked order when that is modelled to save the least gain of the level's work, else the
/// order it has.
#[must_use]
pub(crate) fn chosen(steps: &[LevelStep]) -> Vec<Count> {
    let given: Vec<Count> = (0..steps.len()).map(Count).collect();
    let order = ranked(steps);
    let (before, after) = (modelled(&given, steps), modelled(&order, steps));
    if order == given || after.0 > before.0 * (1.0 - LEAST_GAIN.0) {
        given
    } else {
        order
    }
}
