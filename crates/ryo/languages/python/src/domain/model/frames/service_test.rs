//! The `frames` service's cases, on values alone: what a frame holds of a name after bindings, a deletion and a join.

use super::service::{assign, delete, join, lookup, open};
use super::types::{FrameModel, NameAbsence};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::lexical::types::NamespaceKey;
use crate::domain::model::names::types::NameCode;
use ryo::languages::Position;

/// What the two arms of an alternative do with the one name, after the frame around them bound it.
type Input = (Arm, Arm);
/// How many definitions reach the name after the join, and whether it may be absent.
type Value = Option<(usize, NameAbsence)>;
type Failure = ();

/// What one arm does with the name.
#[derive(Clone, Copy, Debug)]
enum Arm {
    /// Nothing.
    Nothing,
    /// It binds it.
    Binds,
    /// It deletes it.
    Deletes,
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two_arms_that_bind_leave_both_definitions"),
        word: Word::Succeeds,
        input: || (Arm::Binds, Arm::Binds),
        expect: Expect::Value(Some((2, NameAbsence::Present))),
    },
    Case {
        name: CaseName(
            "an_arm_that_does_nothing_leaves_the_outer_definition_beside_the_other_arms",
        ),
        word: Word::Succeeds,
        input: || (Arm::Nothing, Arm::Binds),
        expect: Expect::Value(Some((2, NameAbsence::Present))),
    },
    Case {
        name: CaseName("an_arm_that_deletes_leaves_the_name_possibly_absent"),
        word: Word::Succeeds,
        input: || (Arm::Deletes, Arm::Binds),
        expect: Expect::Value(Some((1, NameAbsence::Absent))),
    },
    Case {
        name: CaseName("two_arms_that_do_nothing_leave_what_was_there"),
        word: Word::Succeeds,
        input: || (Arm::Nothing, Arm::Nothing),
        expect: Expect::Value(Some((1, NameAbsence::Present))),
    },
];

fn subject((first, second): Input) -> Value {
    let key = NamespaceKey {
        scope: Position(0),
        name: NameCode(0),
    };
    let mut model = FrameModel::default();
    let root = open(&mut model, None);
    assign(&mut model, root, key, Position(1), NameAbsence::Present);
    let mut arms = Vec::new();
    for (arm, definition) in [(first, Position(2)), (second, Position(3))] {
        let frame = open(&mut model, Some(root));
        match arm {
            Arm::Nothing => {}
            Arm::Binds => assign(&mut model, frame, key, definition, NameAbsence::Present),
            Arm::Deletes => delete(&mut model, frame, key),
        }
        arms.push(frame);
    }
    join(&mut model, root, &arms);
    lookup(&model, root, key).map(|found| {
        (
            usize::from(found.end).saturating_sub(usize::from(found.start)),
            found.absence,
        )
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
