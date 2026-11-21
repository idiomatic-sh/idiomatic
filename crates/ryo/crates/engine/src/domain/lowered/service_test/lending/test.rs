//! The walk that lends a pack's steps and stages to write, beside the shared walk that lends them to read: on a pack of
//! every fill and on a pack of every step that runs bodies the two meet the same steps and stages in the same order; the
//! lending walk says the unit each is of; and what one pass of it writes the next reads.

use super::super::fixtures::{nested, whole};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowered::service::{each_placed_mut, each_touched};
use crate::domain::lowered::types::{LentPlaced, LoweredPack, WalkedUnit};
use crate::domain::steps::traversal::service::{stage_touched, step_touched};
use crate::domain::steps::types::PlanPoint;
use std::borrow::Cow;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The registers written by what each walk meets, in the order met, on the pack of every fill.
    SameWhole,
    /// The same on the pack of every step that runs bodies.
    SameNested,
    /// The unit of each step and stage of the pack of every fill, in the order met.
    Units,
    /// Each step and stage of the pack of every fill given its place in the walk as its plan point, and the points a second
    /// pass reads.
    Written,
    /// Each step and stage of the pack of every fill numbered as the numbering pass numbers: from nought in each unit.
    Numbered,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// A row.
const fn row(
    name: &'static str,
    input: fn() -> Scenario,
    expected: &'static str,
) -> Case<Input, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input,
        expect: Expect::Value(Cow::Borrowed(expected)),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    row(
        "the_two_walks_meet_the_same_steps_and_stages_in_the_same_order_on_a_pack_of_every_fill",
        || Scenario::SameWhole,
        "both met [1, 2, 3, 4, 5, 6, 10, 11, 12, 13, 14, 15, 16, 17, 18]",
    ),
    row(
        "the_two_walks_meet_the_same_steps_in_the_same_order_through_every_step_that_runs_bodies",
        || Scenario::SameNested,
        "both met [20, 22, 25, 26, 21, 23, 24, 28, 29, 27, 32, 30, 31, 33, 34, 35, 36]",
    ),
    row(
        "the_lending_walk_says_the_unit_each_step_and_stage_is_of",
        || Scenario::Units,
        "Pattern(DefinitionId(0), Count(0)) x2, Rule(SelectedRuleOrdinal(0)) x5, Table(MadeTableId(0)) x3, Table(MadeTableId(1)) x1, Table(MadeTableId(2)) x1, Table(MadeTableId(3)) x2, Table(MadeTableId(4)) x2",
    ),
    row(
        "what_one_pass_of_the_lending_walk_writes_the_next_reads",
        || Scenario::Written,
        "[Some(0), Some(1), Some(2), Some(3), Some(4), Some(5), Some(6), Some(7), Some(8), Some(9), Some(10), Some(11), Some(12), Some(13), Some(14), Some(15)]",
    ),
    row(
        "a_pass_that_numbers_starts_from_nought_when_the_unit_changes",
        || Scenario::Numbered,
        "[0, 1, 0, 1, 2, 3, 4, 0, 1, 2, 0, 0, 0, 1, 0, 1]",
    ),
];

/// The registers written by what the shared walk meets, in the order met.
fn read_writes(pack: &LoweredPack) -> Vec<u32> {
    let mut writes = Vec::new();
    each_touched(pack, &mut |touched| {
        writes.extend(touched.writes.iter().map(|register| register.0));
    });
    writes
}

/// The registers written by what the lending walk meets, in the order met.
fn lent_writes(pack: &mut LoweredPack) -> Vec<u32> {
    let mut writes = Vec::new();
    each_placed_mut(pack, &mut |_, placed| {
        let touched = match &placed {
            LentPlaced::Step(step) => step_touched(&step.step),
            LentPlaced::Stage(stage) => stage_touched(&stage.stage),
        };
        writes.extend(touched.writes.iter().map(|register| register.0));
    });
    writes
}

/// What both walks met when they met the same in the same order; else what each met.
fn same(mut pack: LoweredPack) -> String {
    let (read, lent) = (read_writes(&pack), lent_writes(&mut pack));
    if read == lent {
        format!("both met {read:?}")
    } else {
        format!("the shared walk met {read:?}, the lending walk {lent:?}")
    }
}

/// Each unit the lending walk meets with how many steps or stages of it are met one after the other.
fn units(pack: &mut LoweredPack) -> String {
    let mut met: Vec<(WalkedUnit, usize)> = Vec::new();
    each_placed_mut(pack, &mut |unit, _| match met.last_mut() {
        Some((last, count)) if *last == unit => *count = count.saturating_add(1),
        Some(_) | None => met.push((unit, 1)),
    });
    met.iter()
        .map(|(unit, count)| format!("{unit:?} x{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Gives each step and stage its place in the walk as its plan point; then the points a second pass reads.
fn written(pack: &mut LoweredPack) -> String {
    let mut place = 0_u32;
    each_placed_mut(pack, &mut |_, placed| {
        let point = Some(PlanPoint(place));
        match placed {
            LentPlaced::Step(step) => step.point = point,
            LentPlaced::Stage(stage) => stage.point = point,
        }
        place = place.saturating_add(1);
    });
    let mut read = Vec::new();
    each_placed_mut(pack, &mut |_, placed| {
        let point = match placed {
            LentPlaced::Step(step) => step.point,
            LentPlaced::Stage(stage) => stage.point,
        };
        read.push(point.map(|point| point.0));
    });
    format!("{read:?}")
}

/// Numbers each unit's steps and stages from nought, as the numbering pass does through this walk; then the points read.
fn numbered(pack: &mut LoweredPack) -> String {
    let mut numbering: Option<(WalkedUnit, Option<PlanPoint>)> = None;
    each_placed_mut(pack, &mut |unit, placed| {
        // A unit's steps and stages are met one after the other: its points start from nought when the unit changes.
        if numbering.map(|(of, _)| of) != Some(unit) {
            numbering = Some((unit, Some(PlanPoint(0))));
        }
        let Some((_, next)) = numbering.as_mut() else {
            return;
        };
        let point = *next;
        *next = point
            .and_then(|taken| taken.0.checked_add(1))
            .map(PlanPoint);
        match placed {
            LentPlaced::Step(step) => step.point = point,
            LentPlaced::Stage(stage) => stage.point = point,
        }
    });
    let mut read = Vec::new();
    each_placed_mut(pack, &mut |_, placed| {
        let point = match placed {
            LentPlaced::Step(step) => step.point,
            LentPlaced::Stage(stage) => stage.point,
        };
        read.push(point.map_or(u32::MAX, |point| point.0));
    });
    format!("{read:?}")
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match scenario {
        Scenario::SameWhole => same(whole()),
        Scenario::SameNested => same(nested()),
        Scenario::Units => units(&mut whole()),
        Scenario::Written => written(&mut whole()),
        Scenario::Numbered => numbered(&mut whole()),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
