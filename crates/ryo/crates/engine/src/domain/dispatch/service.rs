//! The services of the `dispatch` concept: the rules of a rule set over one file, in rule order. A branch whose need does
//! not hold in the file is never run: it costs no candidate and no step. The loop's frame answers the leaves of a need that read the
//! file (`types::FileNeeds`).

use super::types::FileNeeds;
use crate::domain::executing::budget::service::{begin_rule, end_rule};
use crate::domain::executing::service::run;
use crate::domain::executing::types::RunMode;
use crate::domain::findings::types::{InstructionCount, ScanCounters};
use crate::domain::needs::service::decided;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::collections::instructions::interface::StepFrame;
use crate::domain::programs::types::Programs;

/// Runs every rule of the rule set over the file, in rule order, each branch when its need holds, and answers what the
/// scan counted. `rules` is the rule set the frame's programs are the body of. A rule whose work passes its allowance in
/// the file is noticed when its work ends, and is never stopped: its rows are all found.
pub(crate) fn dispatch(frame: &mut StepFrame<'_, '_>, rules: &Programs) -> ScanCounters {
    let programs = frame.programs;
    for rule in &programs.code.rules {
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
    ScanCounters {
        steps: work.steps,
        candidates: work.candidates,
        made_rows: InstructionCount(0),
        fill_rows: InstructionCount(0),
        visited: work.visited,
    }
}
