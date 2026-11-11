//! Independent environment algebra: branch order never chooses a supposedly unique winner.
use super::service::{assign, candidates, clear, delete, join, lookup, open, uncertain};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::{
    BindingAbsence, BindingEnvironments, BindingKey, BindingKnowledge,
};
use ryo::languages::{Position, Symbol};
#[derive(Clone, Copy, Debug)]
enum Given {
    Overwrite,
    BothArms,
    DeletedArm,
    MissingArm,
    ZeroIterations,
    UnknownArm,
    DeferredOverlay,
    Reparse,
}
/// What a row is given.
type Input = Given;
type Value = (&'static [Position], BindingAbsence, BindingKnowledge);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("successive stores retain the final declaration identity"),
        word: Word::Succeeds,
        input: || Given::Overwrite,
        expect: Expect::Value((
            &[Position(20)],
            BindingAbsence::Excluded,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("two bound branches retain both declarations and exclude absence"),
        word: Word::Succeeds,
        input: || Given::BothArms,
        expect: Expect::Value((
            &[Position(20), Position(30)],
            BindingAbsence::Excluded,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("bound versus deleted branch retains possible absence"),
        word: Word::Succeeds,
        input: || Given::DeletedArm,
        expect: Expect::Value((
            &[Position(20)],
            BindingAbsence::Possible,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("missing else preserves the incoming declaration"),
        word: Word::Succeeds,
        input: || Given::MissingArm,
        expect: Expect::Value((
            &[Position(10), Position(20)],
            BindingAbsence::Excluded,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("zero-iteration loop preserves an unbound incoming path"),
        word: Word::Succeeds,
        input: || Given::ZeroIterations,
        expect: Expect::Value((
            &[Position(20)],
            BindingAbsence::Possible,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("unknown branch is not proven unbound"),
        word: Word::Succeeds,
        input: || Given::UnknownArm,
        expect: Expect::Value((
            &[Position(20)],
            BindingAbsence::Possible,
            BindingKnowledge::Uncertain,
        )),
    },
    Case {
        name: CaseName("deferred writes cannot mutate completed parent environment"),
        word: Word::Succeeds,
        input: || Given::DeferredOverlay,
        expect: Expect::Value((
            &[Position(10)],
            BindingAbsence::Excluded,
            BindingKnowledge::Known,
        )),
    },
    Case {
        name: CaseName("clear reuses frames without retaining prior binding candidates"),
        word: Word::Succeeds,
        input: || Given::Reparse,
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
        Given::Overwrite | Given::MissingArm | Given::DeferredOverlay | Given::Reparse
    ) {
        assign(&mut state, root, key, Position(10));
    }
    match input {
        Given::Overwrite => assign(&mut state, root, key, Position(20)),
        Given::DeferredOverlay => {
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
        Given::Reparse => {
            clear(&mut state);
            let reused = open(&mut state, None, Position(0));
            assert_eq!(reused, root);
            assert!(lookup(&state, reused, key).is_none());
            assign(&mut state, reused, key, Position(40));
        }
        Given::BothArms
        | Given::DeletedArm
        | Given::MissingArm
        | Given::ZeroIterations
        | Given::UnknownArm => {
            let left = open(&mut state, Some(root), Position(0));
            let right = open(&mut state, Some(root), Position(0));
            assign(&mut state, left, key, Position(20));
            match input {
                Given::BothArms => assign(&mut state, right, key, Position(30)),
                Given::DeletedArm => delete(&mut state, right, key),
                Given::UnknownArm => uncertain(&mut state, right, key),
                Given::MissingArm | Given::ZeroIterations => {}
                Given::Overwrite | Given::DeferredOverlay | Given::Reparse => {
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
