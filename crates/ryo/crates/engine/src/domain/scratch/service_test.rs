//! The `scratch` service's cases: what a file left in a worker's state is gone before the next file, and the room is kept.

use super::service::next_file;
use super::types::{FileFault, IterationCursor, ScanScratch};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::MemoSite;
use crate::domain::filled::errors::FilledError;
use crate::domain::findings::types::InstructionCount;
use crate::domain::memos::service::{asked, keep};
use crate::domain::needs::types::NeedVerdict;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Count, Relation};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

type Input = ();
/// After the next file starts: the cursors, yielded cells, staged cells, key cells, decisions, kept answers, steps and
/// faults left; then how far the store's stamp moved, and the registers kept.
type Value = [usize; 10];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_next_file_holds_nothing_of_the_last_and_keeps_its_registers"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([0, 0, 0, 0, 0, 0, 0, 0, 1, 3]),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let mut scratch = ScanScratch::default();
    let state = &mut scratch.0.state;
    state.registers = vec![CellWord(1); 3];
    state.frames.push(IterationCursor::default());
    state.yielded.push(CellWord(2));
    state.staged.push(CellWord(3));
    state.key.push(CellWord(4));
    state.needs.push(Some(NeedVerdict::Empty));
    let _kept = keep(&mut state.memos, MemoSite(1), &[CellWord(5)], &[], Count(1));
    state.work.steps = InstructionCount(9);
    state.fault = Some(FileFault {
        rule: SelectedRuleOrdinal(0),
        fault: FilledError::Refused {
            relation: Relation::SyntaxError,
            cause: AbsenceCause::NoSuchConstruct,
        },
    });
    let before = scratch.0.store.stamp.0;
    next_file(&mut scratch);
    let state = &scratch.0.state;
    Ok([
        state.frames.len(),
        state.yielded.len(),
        state.staged.len(),
        state.key.len(),
        state.needs.len(),
        usize::from(asked(&state.memos, MemoSite(1), &[CellWord(5)]).is_some()),
        usize::try_from(state.work.steps.0).unwrap_or(usize::MAX),
        usize::from(state.fault.is_some()),
        usize::try_from(scratch.0.store.stamp.0.wrapping_sub(before)).unwrap_or(usize::MAX),
        state.registers.len(),
    ])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
