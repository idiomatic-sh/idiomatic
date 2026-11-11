//! Independent environment algebra: branch order never chooses a supposedly unique winner.
use super::service::{assign, candidates, clear, delete, join, lookup, open, uncertain};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::{
    BindingAbsence, BindingEnvironments, BindingFrameId, BindingKey, BindingKnowledge,
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
/// What a row sees on its way to the final lookup.
#[derive(Debug, PartialEq)]
enum Seen {
    /// The candidates a lookup in the deferred frame answers.
    Candidates(&'static [Position]),
    /// The root frame, then the frame `open` reuses after `clear`.
    Frames(BindingFrameId, BindingFrameId),
    /// Whether a lookup in the reused frame answers a binding.
    Bound(bool),
}
/// What a row is given.
type Input = Given;
type Value = (
    &'static [Position],
    BindingAbsence,
    BindingKnowledge,
    &'static [Seen],
);
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
            &[],
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
            &[],
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
            &[],
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
            &[],
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
            &[],
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
            &[],
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
            &[
                Seen::Candidates(&[Position(10)]),
                Seen::Candidates(&[Position(20)]),
            ],
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
            &[
                Seen::Frames(BindingFrameId(0), BindingFrameId(0)),
                Seen::Bound(false),
            ],
        )),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let mut seen = Vec::new();
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
            let outer = lookup(&state, deferred, key).ok_or(())?;
            seen.push(Seen::Candidates(leak(candidates(&state, outer))));
            assign(&mut state, deferred, key, Position(20));
            let overlay = lookup(&state, deferred, key).ok_or(())?;
            seen.push(Seen::Candidates(leak(candidates(&state, overlay))));
        }
        Given::Reparse => {
            clear(&mut state);
            let reused = open(&mut state, None, Position(0));
            seen.push(Seen::Frames(root, reused));
            seen.push(Seen::Bound(lookup(&state, reused, key).is_some()));
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
        leak(candidates(&state, result)),
        result.absence,
        result.knowledge,
        &*Box::leak(seen.into_boxed_slice()),
    ))
}
/// The candidates as the `'static` slice a row states.
fn leak(positions: &[Position]) -> &'static [Position] {
    Box::leak(positions.to_vec().into_boxed_slice())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
