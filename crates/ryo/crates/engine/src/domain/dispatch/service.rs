//! The services of the `dispatch` concept: the rules of a rule set over one file, in rule order. A branch whose need does
//! not hold in the file is never run: it costs no candidate and no step. The loop's frame answers the leaves of a need that read the
//! file (`executing::sources::types::FileNeeds`).

use crate::domain::executing::budget::service::{begin_rule, end_rule};
use crate::domain::executing::measuring::service::{entered, evaluated, left, used};
use crate::domain::executing::sources::types::FileNeeds;
use crate::domain::findings::types::{InstructionCount, ScanCounters};
use crate::domain::needs::service::decided;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::{RunMode, StepFrame};
use crate::domain::programs::types::Programs;
use ryo_relations::domain::relations::types::Count;

/// Runs every rule of the rule set over the file, in rule order, each branch when its need holds, and answers what the
/// scan counted. `rules` is the rule set the frame's programs are the body of. A rule whose work passes its allowance in
/// the file is noticed when its work ends, and is never stopped: its rows are all found.
pub(crate) fn dispatch(frame: &mut StepFrame<'_, '_>, rules: &Programs) -> ScanCounters {
    let programs = frame.fill.programs;
    for rule in &programs.code.rules {
        frame.state.rule = Some(rule.rule);
        begin_rule(frame.state);
        used(frame.state, rule.unit);
        entered(frame.state, rule.unit);
        let mut rows = 0_u64;
        for branch in &rule.branches {
            if decided(&mut FileNeeds(frame), &programs.code, branch.need) == NeedVerdict::Holds {
                let found = frame.ports.run(frame, branch.entry, RunMode::Rows);
                rows = rows.saturating_add(found.0);
            }
        }
        left(frame.state);
        let candidates = frame.state.budget.candidates.0;
        evaluated(
            frame.state,
            rule.unit,
            Count(usize::try_from(candidates).unwrap_or(usize::MAX)),
            Count(usize::try_from(rows).unwrap_or(usize::MAX)),
        );
        if let Some(notice) = end_rule(frame.state, frame.fill.store, rules, rule) {
            frame.found.noticed(notice);
        }
    }
    let work = frame.state.work;
    let store = &*frame.fill.store;
    let (mut made, mut filled) = (0_u64, 0_u64);
    for slot in store.tables.iter().filter(|slot| slot.stamp == store.stamp) {
        made = made.saturating_add(u64::try_from(slot.work.made.0).unwrap_or(u64::MAX));
        filled = filled.saturating_add(
            u64::try_from(slot.work.read.0.saturating_add(slot.work.given.0)).unwrap_or(u64::MAX),
        );
    }
    ScanCounters {
        steps: work.steps,
        candidates: work.candidates,
        made_rows: InstructionCount(made),
        fill_rows: InstructionCount(filled),
        visited: work.visited,
    }
}
