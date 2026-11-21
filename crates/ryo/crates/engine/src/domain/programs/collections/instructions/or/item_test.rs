//! The `Or` word's cases: each staged row is bound to the word's outputs in their order, and the staged room is given
//! back when the rows are done.

use super::item::Or;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::controlling::service::or;
use crate::domain::code::controlling::types::OrWord;
use crate::domain::code::records::service::push_list;
use crate::domain::code::types::{
    CodeEntry, MemoSite, PoolAt, ProgramCode, Short, WordFlag, WordSlot,
};
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::BodyAnswer;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

/// How many rows of two cells are staged after one cell of another word.
type Input = usize;
/// Registers 0 and 1 after each row bound; then 99 and how many staged cells are left.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases: the outputs are register 1, then register 0; the staged rows are (5, 6) and (7, 8).
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_row_is_bound_to_the_outputs_in_their_order"),
        word: Word::Succeeds,
        input: || 2,
        expect: Expect::Value(Cow::Borrowed(&[6, 5, 8, 7, 99, 1])),
    },
    Case {
        name: CaseName("no_row_binds_nothing_and_gives_the_room_back"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(Cow::Borrowed(&[99, 1])),
    },
];

fn subject(rows: Input) -> Result<Value, Failure> {
    let mut code = ProgramCode {
        pool: vec![WordSlot(0)],
        ..ProgramCode::default()
    };
    let outputs = push_list(&mut code.pool, &[WordSlot(1), WordSlot(0)]);
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = held(2);
    held.state.staged.push(CellWord(77));
    held.state.staged.extend(
        [5_u64, 6, 7, 8]
            .iter()
            .take(rows * 2)
            .map(|cell| CellWord(*cell)),
    );
    held.state.answer = BodyAnswer {
        first: Count(1),
        rows: Count(rows),
    };
    let word = or(OrWord {
        flag: WordFlag(0),
        bodies: Short(0),
        records: PoolAt(0),
        outputs,
        memo: MemoSite(0),
    });
    let (bound, _found) = framed(&programs, &mut held, &Idle, |frame| {
        let mut bound = Vec::new();
        if Or::run(frame, word, CodeEntry(3)) != Flow::Back {
            return bound;
        }
        let Some(mut cursor) = frame.state.frames.last().copied() else {
            return bound;
        };
        while Or::next(frame, word, &mut cursor).is_some() {
            bound.extend([frame.state.registers[0].0, frame.state.registers[1].0]);
        }
        bound.push(99);
        bound.push(u64::try_from(frame.state.staged.len()).unwrap_or(u64::MAX));
        bound
    });
    Ok(Cow::Owned(bound))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
