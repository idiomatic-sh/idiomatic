//! The `Fork` word's cases: the loop goes on at each slot's entry in turn, and then at none.

use super::item::Fork;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::controlling::service::fork;
use crate::domain::code::controlling::types::ForkWord;
use crate::domain::code::records::service::push_list;
use crate::domain::code::types::{CodeEntry, ProgramCode, Short, WordSlot};
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use std::borrow::Cow;

/// The entries of the slots.
type Input = &'static [u32];
/// The entries the loop goes on at, in order; then 99.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_slot_runs_in_turn"),
        word: Word::Succeeds,
        input: || &[10, 20, 14],
        expect: Expect::Value(Cow::Borrowed(&[10, 20, 14, 99])),
    },
    Case {
        name: CaseName("a_fork_of_no_slot_runs_nothing"),
        word: Word::Succeeds,
        input: || &[],
        expect: Expect::Value(Cow::Borrowed(&[99])),
    },
];

fn subject(entries: Input) -> Result<Value, Failure> {
    let mut code = ProgramCode {
        pool: vec![WordSlot(0)],
        ..ProgramCode::default()
    };
    let slots: Vec<WordSlot> = entries.iter().map(|each| WordSlot(*each)).collect();
    let word = fork(ForkWord {
        slots: Short(u16::try_from(entries.len()).unwrap_or(u16::MAX)),
        entries: push_list(&mut code.pool, &slots),
    });
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = held(1);
    let (ran, _found) = framed(&programs, &mut held, &Idle, |frame| {
        let mut ran = Vec::new();
        if Fork::run(frame, word, CodeEntry(3)) != Flow::Back {
            return ran;
        }
        let Some(mut cursor) = frame.state.frames.last().copied() else {
            return ran;
        };
        while let Some(onward) = Fork::next(frame, word, &mut cursor) {
            ran.push(u64::from(onward.0));
        }
        ran.push(99);
        ran
    });
    Ok(Cow::Owned(ran))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
