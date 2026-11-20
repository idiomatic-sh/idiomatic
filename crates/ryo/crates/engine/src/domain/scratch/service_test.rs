//! The `scratch` service's cases: what a file left in a worker's state is gone before the next file.

use super::service::clear_file;
use super::types::ScanScratch;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::types::PresenceWord;

type Input = ();
/// The registers, filled bits, made rows and present words left after the clear.
type Value = [usize; 4];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_cleared_scratch_holds_nothing_of_the_last_file"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([0, 0, 0, 0]),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let mut scratch = ScanScratch::default();
    scratch.0.registers.push(CellWord(1));
    scratch.0.filled.push(super::types::FilledWord(1));
    scratch.0.made.push(vec![CellWord(2)]);
    scratch.0.present.0.push(PresenceWord(4));
    clear_file(&mut scratch);
    let state = &scratch.0;
    Ok([
        state.registers.len(),
        state.filled.len(),
        state.made.iter().map(Vec::len).sum(),
        state.present.0.len(),
    ])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
