//! The `Call` word's cases: each staged row's cells are bound to the word's outputs in turn, and the staged room is
//! given back.

use super::item::Call;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::deciding::service::call;
use crate::domain::code::deciding::types::CallWord;
use crate::domain::code::records::service::push_list;
use crate::domain::code::types::{CodeEntry, PoolAt, ProgramCode, Short, WordFlag, WordSlot};
use crate::domain::programs::collections::instructions::interface::InstructionsItem;
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::BodyAnswer;
use crate::domain::steps::types::RegexId;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

/// The cells staged before the call's, the cells the call staged, and how many rows they are; the word has two outputs
/// (registers 1 and 2) or none.
type Input = (&'static [u64], &'static [u64], usize, bool);
/// The cells bound at each turn; then 99, where the loop goes on (the entry after the word, once), and how many staged
/// cells are left.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_row_of_two_cells_is_bound_in_turn_and_the_room_given_back"),
        word: Word::Succeeds,
        input: || (&[40], &[5, 6, 7, 8], 2, true),
        expect: Expect::Value(Cow::Borrowed(&[5, 6, 77, 7, 8, 77, 99, 6, 1])),
    },
    Case {
        name: CaseName("a_row_of_no_cell_lets_the_candidate_go_on_once"),
        word: Word::Succeeds,
        input: || (&[40], &[], 1, false),
        expect: Expect::Value(Cow::Borrowed(&[0, 0, 77, 99, 6, 1])),
    },
    Case {
        name: CaseName("a_call_that_gave_no_row_binds_nothing"),
        word: Word::Succeeds,
        input: || (&[40], &[], 0, true),
        expect: Expect::Value(Cow::Borrowed(&[99, 0, 1])),
    },
];

fn subject((before, given, rows, outputs): Input) -> Result<Value, Failure> {
    let mut code = ProgramCode {
        pool: vec![WordSlot(0)],
        ..ProgramCode::default()
    };
    let registers: &[WordSlot] = if outputs {
        &[WordSlot(1), WordSlot(2)]
    } else {
        &[]
    };
    let listed: PoolAt = push_list(&mut code.pool, registers);
    let word = call(CallWord {
        flag: WordFlag(0),
        meaning: Short(1),
        arguments: PoolAt(0),
        outputs: listed,
        regex: RegexId(0),
    });
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = held(3);
    held.state.staged = before
        .iter()
        .chain(given)
        .map(|cell| CellWord(*cell))
        .collect();
    held.state.answer = BodyAnswer {
        first: Count(before.len()),
        rows: Count(rows),
    };
    let (bound, _found) = framed(&programs, &mut held, &Idle, |frame| {
        let _flow = Call::run(frame, word, CodeEntry(5));
        let mut cursor = frame.state.frames.pop().unwrap_or_default();
        let (mut bound, mut onward) = (Vec::new(), 0_u64);
        while let Some(next) = Call::next(frame, word, &mut cursor) {
            bound.extend(frame.state.registers.iter().skip(1).map(|cell| cell.0));
            bound.push(77);
            onward = u64::from(next.0);
        }
        bound.extend([99, onward]);
        bound
    });
    let mut all = bound;
    all.push(u64::try_from(held.state.staged.len()).unwrap_or(u64::MAX));
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
