//! A node the file touched is listed once however often it is touched; the list is taken ascending, and a node taken is
//! untouched again, so the next file lists it anew.
use super::value::TouchedNodes;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::PlanNodeId;
use std::borrow::Cow;

/// What a case touches.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Nodes 4, 1, 4, then 2.
    Repeated,
    /// Node 3, taken, then nodes 3 and 0.
    AfterTaken,
}
type Input = Given;
/// The node ids taken last.
type Value = Cow<'static, [u32]>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_touched_twice_is_listed_once_ascending"),
        word: Word::Succeeds,
        input: || Given::Repeated,
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 4])),
    },
    Case {
        name: CaseName("a_node_taken_is_untouched_again"),
        word: Word::Succeeds,
        input: || Given::AfterTaken,
        expect: Expect::Value(Cow::Borrowed(&[0, 3])),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    let mut touched = TouchedNodes::default();
    let ids: &[u32] = match given {
        Given::Repeated => &[4, 1, 4, 2],
        Given::AfterTaken => {
            touched.touched(PlanNodeId(3));
            let taken = touched.taken();
            touched.given_back(taken);
            &[3, 0]
        }
    };
    for id in ids {
        touched.touched(PlanNodeId(*id));
    }
    Ok(Cow::Owned(
        touched.taken().iter().map(|node| node.0).collect(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
