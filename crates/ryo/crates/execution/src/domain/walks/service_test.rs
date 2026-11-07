//! The `walks` service's cases: which elements a walk is told are answers.

use super::service::allows;
use super::types::{Admit, Marked};
use crate::domain::operating::errors::OperatingError;
use crate::domain::shared_walk::service_test::with_context;
use crate::domain::table_cells::service_test::{SmallTree, rows_over, small_tree};
use crate::domain::table_model::types::Verdict;
use ryo_relations::domain::relations::types::Position;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases asks: how the elements are told to be answers, and the element asked of.
#[derive(Clone, Copy)]
enum Told {
    /// Every element is an answer.
    All(u32),
    /// The elements marked by position are, and the element asked of is at this place.
    Marked(&'static [bool], u32),
}

/// What a row of the cases asks.
type Input = Told;
/// Whether the element is an answer.
type Value = Verdict;
/// What the walk refuses with.
type Failure = OperatingError;

fn subject(input: Input) -> Result<Value, Failure> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let tree = rows_over(&elements);
    let (mut admit, found) = match input {
        Told::All(found) => (Admit::All, found),
        Told::Marked(marks, found) => (Admit::Marks(Marked(marks.to_vec())), found),
    };
    with_context(&mut symbols, |context| {
        allows(&mut admit, &tree, Position(found), context)
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("with_every_element_told_an_answer_each_is_one"),
        word: Word::Succeeds,
        input: || Told::All(2),
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("a_marked_element_is_an_answer"),
        word: Word::Succeeds,
        input: || Told::Marked(&[true, false], 0),
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("an_unmarked_element_is_no_answer"),
        word: Word::Succeeds,
        input: || Told::Marked(&[true, false], 1),
        expect: Expect::Value(Verdict::Fails),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
