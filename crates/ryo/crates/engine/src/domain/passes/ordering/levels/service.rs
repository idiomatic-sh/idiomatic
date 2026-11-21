//! The service of the `levels` part: what one round of the pass does, read off the pack before anything moves. A level is
//! the steps of one program: a pattern's call form, a rule's branch, an inline body, a slot. A round is every level at one
//! depth; for each it holds the order the level takes, and for the lending walk what to do at each step it lends. The
//! order of a level is decided from the runs the sample counted at each step's plan point: a step's rate is the runs of
//! the step after it over its own, its work is its own run, the runs of the steps of its bodies and what the sample says
//! its uses cost, for each of its runs. What may move: the steps the sample reached, from the first that is not an anchor
//! to the last before a step that ends a row or hands the level on; the level's last step keeps its place.

use crate::domain::code::types::UnitKey;
use crate::domain::lowered::types::{LoweredPack, PatternForm};
use crate::domain::passes::ordering::constants::{LEAST_MOVABLE, UNPASSED_ROWS};
use crate::domain::passes::ordering::costs::service::{counted, step_facts};
use crate::domain::passes::ordering::ranking::service::chosen;
use crate::domain::passes::ordering::types::{
    LevelRole, LevelStep, OrderRound, PassRate, RoundEvent, RunWork, StepPlacement, UnitOrder,
    UseCosts,
};
use crate::domain::statistics::service::unit_of;
use crate::domain::statistics::types::{MeasuredUnit, OrderVerdict, Statistics};
use crate::domain::steps::service::step_class;
use crate::domain::steps::traversal::service::{each_step, step_touched};
use crate::domain::steps::traversal::types::{Touched, TouchedBody};
use crate::domain::steps::types::{CostClass, PlacedStep, ProgramStep, StepProgram};
use ryo_relations::domain::relations::types::Count;

/// Every step of a step's inline bodies, each once, with what it touches.
fn each_nested<'a>(step: &'a ProgramStep, visit: &mut impl FnMut(&'a PlacedStep, &Touched<'a>)) {
    for body in step_touched(step).bodies {
        match body {
            TouchedBody::Inline(inner) => each_step(inner, visit),
            TouchedBody::Definition(_) => {}
        }
    }
}

/// How many steps a step's inline bodies hold, through theirs.
#[must_use]
pub(crate) fn nested_steps(step: &ProgramStep) -> Count {
    let mut steps = 0_usize;
    each_nested(step, &mut |_, _| steps = steps.saturating_add(1));
    Count(steps)
}

/// The runs the sample counted at a step's plan point; nought for a step with no point, or one the sample did not hold.
fn runs(measured: &MeasuredUnit, placed: &PlacedStep) -> Count {
    let place = placed
        .point
        .map(|point| usize::try_from(point.0).unwrap_or(usize::MAX));
    place
        .and_then(|place| measured.points.get(place).copied())
        .unwrap_or_default()
}

/// The counted work of every run of one step, its bodies left out: each run is one, and what each run's uses cost: a use
/// of a definition, a read of a made table.
fn own_work(
    order: UnitOrder<'_>,
    measured: &MeasuredUnit,
    placed: &PlacedStep,
    touched: &Touched<'_>,
) -> RunWork {
    let UseCosts {
        definitions,
        tables,
    } = order.costs;
    let used = touched.bodies.iter().map(|body| match body {
        TouchedBody::Definition(definition) => {
            definitions.get(usize::try_from(definition.0).unwrap_or(usize::MAX))
        }
        TouchedBody::Inline(_) => None,
    });
    let facts = step_facts(&placed.step);
    let read = facts
        .tables
        .iter()
        .map(|table| tables.get(usize::try_from(table.0).unwrap_or(usize::MAX)));
    let each = used
        .chain(read)
        .flatten()
        .fold(0.0, |sum, cost| sum + cost.0);
    RunWork(counted(runs(measured, placed)).0 * (1.0 + each))
}

/// What the pass knows of the step at `place` of a level: the registers it and its bodies read and write, its rate and
/// its work for each run. None for a step with no step after it.
fn level_step(
    order: UnitOrder<'_>,
    measured: &MeasuredUnit,
    program: &StepProgram,
    place: Count,
) -> Option<LevelStep> {
    let placed = program.0.get(place.0)?;
    let after = program.0.get(place.0.checked_add(1)?)?;
    let touched = step_touched(&placed.step);
    let (mut reads, mut writes) = (touched.reads.clone(), touched.writes.clone());
    let mut work = own_work(order, measured, placed, &touched).0;
    each_nested(&placed.step, &mut |inner, inner_touched| {
        reads.extend(&inner_touched.reads);
        writes.extend(&inner_touched.writes);
        work += own_work(order, measured, inner, inner_touched).0;
    });
    let (own, next) = (
        counted(runs(measured, placed)).0,
        counted(runs(measured, after)).0,
    );
    // A step nothing passed on the sample is not taken as never passing.
    let rate = if next > 0.0 {
        next / own.max(1.0)
    } else {
        UNPASSED_ROWS.0 / (own + 1.0)
    };
    Some(LevelStep {
        reads,
        writes,
        rate: PassRate(rate),
        work: RunWork(work / own.max(1.0)),
    })
}

/// The order a level takes: for each place, the place its step holds now. A unit with no measured order keeps every level
/// as it is. The steps that may move are a run of the level: after the anchor of a rule's branch and any file-sized step,
/// up to the first step the sample did not reach, the first that ends a row or hands the level on, or the last.
#[must_use]
pub(crate) fn level_order(
    program: &StepProgram,
    role: LevelRole,
    order: UnitOrder<'_>,
) -> Vec<Count> {
    let steps = &program.0;
    let given: Vec<Count> = (0..steps.len()).map(Count).collect();
    let Some(measured) = order.measured else {
        return given;
    };
    let anchors = match role {
        LevelRole::Branch => 1,
        LevelRole::Keyed => 0,
    };
    let sized = |placed: &&PlacedStep| step_class(&placed.step) == CostClass::File;
    let start = anchors + steps.iter().skip(anchors).take_while(sized).count();
    let movable = |placed: &&PlacedStep| {
        runs(measured, placed).0 > 0
            && step_class(&placed.step) != CostClass::File
            && step_facts(&placed.step).placement == StepPlacement::Free
    };
    let last = steps.len().saturating_sub(1);
    let run = steps
        .iter()
        .take(last)
        .skip(start)
        .take_while(movable)
        .count();
    if run < LEAST_MOVABLE.0 {
        return given;
    }
    let level: Option<Vec<LevelStep>> = (start..start + run)
        .map(|place| level_step(order, measured, program, Count(place)))
        .collect();
    let Some(level) = level else {
        return given;
    };
    let moved = chosen(&level);
    let before = (0..start).map(Count);
    let within = moved.iter().map(|place| Count(start + place.0));
    let after = (start + run..steps.len()).map(Count);
    before.chain(within).chain(after).collect()
}

/// How many plan points the programs of one unit hold: one more than the greatest point a step of them bears.
fn unit_points<'a>(programs: impl Iterator<Item = &'a StepProgram>) -> Count {
    let mut reach = 0_usize;
    for program in programs {
        each_step(program, &mut |placed, _| {
            if let Some(point) = placed.point {
                let place = usize::try_from(point.0).unwrap_or(usize::MAX);
                reach = reach.max(place.saturating_add(1));
            }
        });
    }
    Count(reach)
}

/// The unit the statistics measured under a name, when its measured order may be taken: the sample measured as many plan
/// points as the unit holds now (statistics of another lowering of the node are stale, and ignored), and its order is not
/// denied.
fn measured_as<'s, 'a>(
    statistics: &'s Statistics,
    key: &UnitKey,
    programs: impl Iterator<Item = &'a StepProgram>,
) -> Option<&'s MeasuredUnit> {
    let unit = unit_of(statistics, key)?;
    let fresh = unit.points.len() == unit_points(programs).0;
    let taken = match unit.order {
        OrderVerdict::Learned => true,
        OrderVerdict::Denied => false,
    };
    (fresh && taken).then_some(unit)
}

/// Reads one program into the round: at the round's depth its order and a move for each of its steps; above it, each step
/// passed and its inline bodies read one level down, in the order the lending walk goes into them.
fn planned(
    program: &StepProgram,
    role: LevelRole,
    at: Count,
    depth: Count,
    order: UnitOrder<'_>,
    round: &mut OrderRound,
) {
    if at == depth {
        round.levels.push(level_order(program, role, order));
        let moves = program.0.iter().map(|_| RoundEvent::Moved);
        round.events.extend(moves);
        return;
    }
    for placed in &program.0 {
        round.events.push(RoundEvent::Passed);
        for body in step_touched(&placed.step).bodies {
            match body {
                TouchedBody::Inline(inner) => {
                    let below = Count(at.0.saturating_add(1));
                    planned(inner, LevelRole::Keyed, below, depth, order, round);
                }
                TouchedBody::Definition(_) => {}
            }
        }
    }
}

/// The round at one depth, read in the lending walk's order: the call forms of the definitions' patterns, then the rules'
/// branches. A round with no level means no program is that deep.
#[must_use]
pub(crate) fn round_at(
    pack: &LoweredPack,
    statistics: &Statistics,
    costs: &UseCosts,
    depth: Count,
) -> OrderRound {
    let mut round = OrderRound::default();
    for definition in &pack.definitions {
        for pattern in &definition.patterns {
            match &pattern.form {
                PatternForm::Call(program) => {
                    let key = UnitKey::Pattern(
                        definition.node,
                        pattern.bound.clone(),
                        pattern.collected.clone(),
                    );
                    let order = UnitOrder {
                        costs,
                        measured: measured_as(statistics, &key, [program].into_iter()),
                    };
                    planned(
                        program,
                        LevelRole::Keyed,
                        Count(0),
                        depth,
                        order,
                        &mut round,
                    );
                }
                PatternForm::Table(_) => {}
            }
        }
    }
    for rule in &pack.rules {
        let programs = || rule.branches.iter().map(|branch| &branch.program);
        let order = UnitOrder {
            costs,
            measured: measured_as(statistics, &UnitKey::Rule(rule.node), programs()),
        };
        for program in programs() {
            planned(
                program,
                LevelRole::Branch,
                Count(0),
                depth,
                order,
                &mut round,
            );
        }
    }
    round
}
