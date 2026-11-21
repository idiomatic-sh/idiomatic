//! The `Nearest` word's cases: the element the loop staged is bound and the candidate goes on; with none staged the
//! candidate ends; the staged room is given back either way.

use super::item::Nearest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{CodeEntry, PoolAt, WordFlag};
use crate::domain::code::walking::service::nearest;
use crate::domain::code::walking::types::NearestWord;
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::BodyAnswer;
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The cells staged, the last of them the element found when the walk found one; and whether it found one.
type Input = (&'static [u64], bool);
/// Whether the candidate goes on (1) or ends (0), the cell the word's register holds, and the staged cells left.
type Value = [u64; 3];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_element_found_is_bound_and_the_candidate_goes_on"),
        word: Word::Succeeds,
        input: || (&[40, 9], true),
        expect: Expect::Value([1, 9, 1]),
    },
    Case {
        name: CaseName("with_no_element_found_the_candidate_ends"),
        word: Word::Succeeds,
        input: || (&[40], false),
        expect: Expect::Value([0, 0, 1]),
    },
];

fn subject((staged, found): Input) -> Result<Value, Failure> {
    let programs = ProgramsEngine::default();
    let mut held = held(2);
    held.state.staged = staged.iter().map(|cell| CellWord(*cell)).collect();
    held.state.answer = BodyAnswer {
        first: Count(staged.len().saturating_sub(usize::from(found))),
        rows: Count(usize::from(found)),
    };
    let word = nearest(NearestWord {
        flag: WordFlag(0),
        start: Register(0),
        out: Register(1),
        operands: PoolAt(0),
    });
    let (flow, _found) = framed(&programs, &mut held, &Idle, |frame| {
        Nearest::run(frame, word, CodeEntry(1))
    });
    Ok([
        u64::from(flow == Flow::Next),
        held.state.registers.get(1).map_or(0, |cell| cell.0),
        u64::try_from(held.state.staged.len()).unwrap_or(u64::MAX),
    ])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
