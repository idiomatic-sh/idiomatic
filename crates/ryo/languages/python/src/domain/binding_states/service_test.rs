//! Independent environment algebra: branch order never chooses a supposedly unique winner.
use super::service::{assign, candidates, clear, delete, join, lookup, open, uncertain};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::{
    BindingAbsence, BindingEnvironments, BindingKey, BindingKnowledge,
};
use ryo::languages::{Position, Symbol};
#[derive(Clone, Copy, Debug)]
enum Input {
    Overwrite,
    BothArms,
    DeletedArm,
    MissingArm,
    ZeroIterations,
    UnknownArm,
    DeferredOverlay,
    Reparse,
}
type Value = (&'static [Position], BindingAbsence, BindingKnowledge);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("successive stores retain the final declaration identity"),
        word: Word::Succeeds,
        input: || Input::Overwrite,
        expect: Expect::Value((
            &[Position(20)],
            BindingAbsence::Excluded,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("two bound branches retain both declarations and exclude absence"),
        word: Word::Succeeds,
        input: || Input::BothArms,
        expect: Expect::Value((
            &[Position(20), Position(30)],
            BindingAbsence::Excluded,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("bound versus deleted branch retains possible absence"),
        word: Word::Succeeds,
        input: || Input::DeletedArm,
        expect: Expect::Value((
            &[Position(20)],
            BindingAbsence::Possible,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("missing else preserves the incoming declaration"),
        word: Word::Succeeds,
        input: || Input::MissingArm,
        expect: Expect::Value((
            &[Position(10), Position(20)],
            BindingAbsence::Excluded,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("zero-iteration loop preserves an unbound incoming path"),
        word: Word::Succeeds,
        input: || Input::ZeroIterations,
        expect: Expect::Value((
            &[Position(20)],
            BindingAbsence::Possible,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("unknown branch is not proven unbound"),
        word: Word::Succeeds,
        input: || Input::UnknownArm,
        expect: Expect::Value((
            &[Position(20)],
            BindingAbsence::Possible,
            BindingKnowledge::Uncertain,
        )),
    },
    Case {
        name: CaseName("deferred writes cannot mutate completed parent environment"),
        word: Word::Succeeds,
        input: || Input::DeferredOverlay,
        expect: Expect::Value((
            &[Position(10)],
            BindingAbsence::Excluded,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("clear reuses frames without retaining prior binding candidates"),
        word: Word::Succeeds,
        input: || Input::Reparse,
        expect: Expect::Value((
            &[Position(40)],
            BindingAbsence::Excluded,
            BindingKnowledge::Known,
        )),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let mut state = BindingEnvironments::default();
    let root = open(&mut state, None, Position(0));
    let key = BindingKey {
        scope: Position(0),
        name: Symbol(7),
    };
    if matches!(
        input,
        Input::Overwrite | Input::MissingArm | Input::DeferredOverlay | Input::Reparse
    ) {
        assign(&mut state, root, key, Position(10));
    }
    match input {
        Input::Overwrite => assign(&mut state, root, key, Position(20)),
        Input::DeferredOverlay => {
            let deferred = open(&mut state, Some(root), Position(50));
            assert_eq!(
                candidates(
                    &state,
                    lookup(&state, deferred, key).expect("outer declaration")
                ),
                &[Position(10)]
            );
            assign(&mut state, deferred, key, Position(20));
            assert_eq!(
                candidates(
                    &state,
                    lookup(&state, deferred, key).expect("phase-local overlay")
                ),
                &[Position(20)]
            );
        }
        Input::Reparse => {
            clear(&mut state);
            let reused = open(&mut state, None, Position(0));
            assert_eq!(reused, root);
            assert!(lookup(&state, reused, key).is_none());
            assign(&mut state, reused, key, Position(40));
        }
        Input::BothArms
        | Input::DeletedArm
        | Input::MissingArm
        | Input::ZeroIterations
        | Input::UnknownArm => {
            let left = open(&mut state, Some(root), Position(0));
            let right = open(&mut state, Some(root), Position(0));
            assign(&mut state, left, key, Position(20));
            match input {
                Input::BothArms => assign(&mut state, right, key, Position(30)),
                Input::DeletedArm => delete(&mut state, right, key),
                Input::UnknownArm => uncertain(&mut state, right, key),
                Input::MissingArm | Input::ZeroIterations => {}
                Input::Overwrite | Input::DeferredOverlay | Input::Reparse => {
                    return Err(());
                }
            }
            join(&mut state, root, &[right, left]);
        }
    }
    let result = lookup(&state, root, key).ok_or(())?;
    Ok((
        &*Box::leak(candidates(&state, result).to_vec().into_boxed_slice()),
        result.absence,
        result.knowledge,
    ))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
