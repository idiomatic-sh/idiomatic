//! The services of the `budget` part: the run-time work budget, counted in rows. A pattern's answer keeps its own program's
//! work and the list of the answers it asked directly; a rule is charged an answer the first time it reaches it in the
//! file, by a stamp, and follows the list, so each unit a rule reaches is charged to it once, exactly as when the rule
//! runs alone. One compare when a rule's work in a file ends; a rule over its allowance is noticed and never stopped.

use super::types::UnitCaller;
use crate::domain::code::types::{RuleCode, WordSlot};
use crate::domain::executing::constants::ALLOWANCE;
use crate::domain::findings::types::{HeaviestWork, InstructionCount, WorkNotice, WorkVerdict};
use crate::domain::memos::service::{charge_mut, reached_at, set_charge};
use crate::domain::memos::types::{EntryCharge, MemoId, RuleStamp};
use crate::domain::programs::service::unit_step;
use crate::domain::programs::types::Programs;
use crate::domain::scratch::types::{LoopState, RuleBudget, UnitTally};
use ryo_relations::domain::relations::types::Count;

/// Starts a pattern's evaluation: its own work is counted apart from its caller's.
pub(crate) fn begin_unit(state: &mut LoopState) -> UnitCaller {
    let caller = UnitCaller {
        visited: state.budget.visited,
        asked: Count(state.budget.reaching.len()),
    };
    state.budget.visited = InstructionCount(0);
    state.budget.depth = Count(state.budget.depth.0.saturating_add(1));
    caller
}

/// Ends a pattern's evaluation: the answer keeps its own work, its unit and the answers it asked directly, and the caller's
/// count goes on where it was.
pub(crate) fn end_unit(state: &mut LoopState, caller: UnitCaller, answer: MemoId, unit: WordSlot) {
    let LoopState { memos, budget, .. } = state;
    set_charge(
        memos,
        answer,
        EntryCharge {
            work: budget.visited,
            unit,
            ..EntryCharge::default()
        },
        budget.reaching.get(caller.asked.0..).unwrap_or_default(),
    );
    budget.reaching.truncate(caller.asked.0);
    budget.visited = caller.visited;
    budget.depth = Count(budget.depth.0.saturating_sub(1));
}

/// One unit's tally for the running rule, made nought when it was another rule's.
fn tally(budget: &mut RuleBudget, unit: WordSlot) -> Option<&mut UnitTally> {
    let place = usize::try_from(unit.0).unwrap_or(usize::MAX);
    if budget.tallies.len() <= place {
        budget
            .tallies
            .resize(place.saturating_add(1), UnitTally::default());
    }
    let stamp = budget.stamp;
    let kept = budget.tallies.get_mut(place)?;
    if kept.stamp != stamp {
        *kept = UnitTally {
            stamp,
            ..UnitTally::default()
        };
    }
    Some(kept)
}

/// Charges the running rule an answer and everything the answer's lists reach, each once.
fn charge(state: &mut LoopState, answer: MemoId) {
    let LoopState { memos, budget, .. } = state;
    budget.pending.push(answer);
    while let Some(next) = budget.pending.pop() {
        let Some(entry) = charge_mut(memos, next) else {
            continue;
        };
        if entry.charged == budget.stamp {
            continue;
        }
        entry.charged = budget.stamp;
        let (work, unit, asked) = (entry.work, entry.unit, entry.reached);
        budget.charged = InstructionCount(budget.charged.0.saturating_add(work.0));
        budget.keys = InstructionCount(budget.keys.0.saturating_add(1));
        if let Some(kept) = tally(budget, unit) {
            kept.keys = InstructionCount(kept.keys.0.saturating_add(1));
            kept.visited = InstructionCount(kept.visited.0.saturating_add(work.0));
        }
        budget.pending.extend_from_slice(reached_at(memos, asked));
    }
}

/// A pattern's answer was asked, kept or just evaluated. Asked by the rule's own program, the rule is charged it; asked
/// inside a pattern's evaluation, it goes on that pattern's list, and the rule is charged it when it reaches the pattern.
pub(crate) fn reach(state: &mut LoopState, answer: MemoId) {
    if state.budget.depth.0 == 0 {
        charge(state, answer);
    } else {
        state.budget.reaching.push(answer);
    }
}

/// Starts a rule's work in a file: nothing is charged to it yet.
pub(crate) fn begin_rule(state: &mut LoopState) {
    let budget = &mut state.budget;
    budget.stamp = RuleStamp(budget.stamp.0.wrapping_add(1));
    budget.visited = InstructionCount(0);
    budget.candidates = InstructionCount(0);
    budget.charged = InstructionCount(0);
    budget.keys = InstructionCount(0);
    budget.depth = Count(0);
    budget.reaching.clear();
}

/// Ends a rule's work in a file: the one compare. Its work is the rows its own program's iterations gave and the work of
/// every unit charged to it; its allowance is `ALLOWANCE` times its candidates and the keys it asked of call patterns.
/// Past it, the notice names the unit that did the most counted work, the first in unit order on a tie.
pub(crate) fn end_rule(
    state: &LoopState,
    programs: &Programs,
    rule: &RuleCode,
) -> Option<WorkNotice> {
    let budget = &state.budget;
    let work = budget.visited.0.saturating_add(budget.charged.0);
    let allowance = ALLOWANCE
        .0
        .saturating_mul(budget.candidates.0.saturating_add(budget.keys.0));
    if work <= allowance {
        return None;
    }
    let mut heaviest = (
        rule.unit,
        budget.visited,
        HeaviestWork::Program {
            candidates: budget.candidates,
            visited: budget.visited,
        },
    );
    for (place, kept) in budget.tallies.iter().enumerate() {
        let unit = WordSlot(u32::try_from(place).unwrap_or(u32::MAX));
        let heavier = kept.visited.0 > heaviest.1.0
            || (kept.visited.0 == heaviest.1.0 && unit.0 < heaviest.0.0);
        if kept.stamp == budget.stamp && heavier {
            heaviest = (
                unit,
                kept.visited,
                HeaviestWork::Calls {
                    keys: kept.keys,
                    visited: kept.visited,
                },
            );
        }
    }
    let step = programs
        .0
        .code
        .units
        .get(usize::try_from(heaviest.0.0).unwrap_or(usize::MAX))
        .map(|unit| unit_step(programs, rule.rule, &unit.key))?;
    Some(WorkNotice {
        rule: rule.rule,
        verdict: WorkVerdict::Over,
        work: InstructionCount(work),
        allowance: InstructionCount(allowance),
        step,
        heaviest: heaviest.2,
    })
}
