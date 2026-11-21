//! The services of the `dispatch` concept: the rules of a rule set over one file, in rule order. A branch whose need does
//! not hold in the file is never run: it costs no candidate and no step. The loop's frame answers the leaves of a need that read the
//! file (`executing::sources::types::FileNeeds`).

use crate::domain::executing::budget::service::{begin_rule, end_rule};
use crate::domain::executing::service::run;
use crate::domain::executing::sources::types::FileNeeds;
use crate::domain::findings::types::{InstructionCount, ScanCounters};
use crate::domain::needs::service::decided;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::{RunMode, StepFrame};
use crate::domain::programs::types::Programs;

/// Runs every rule of the rule set over the file, in rule order, each branch when its need holds, and answers what the
/// scan counted. `rules` is the rule set the frame's programs are the body of. A rule whose work passes its allowance in
/// the file is noticed when its work ends, and is never stopped: its rows are all found.
pub(crate) fn dispatch(frame: &mut StepFrame<'_, '_>, rules: &Programs) -> ScanCounters {
    let programs = frame.fill.programs;
    for rule in &programs.code.rules {
        frame.state.rule = Some(rule.rule);
        begin_rule(frame.state);
        for branch in &rule.branches {
            if decided(&mut FileNeeds(frame), &programs.code, branch.need) == NeedVerdict::Holds {
                let _rows: InstructionCount = run(frame, branch.entry, RunMode::Rows);
            }
        }
        if let Some(notice) = end_rule(frame.state, rules, rule) {
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
