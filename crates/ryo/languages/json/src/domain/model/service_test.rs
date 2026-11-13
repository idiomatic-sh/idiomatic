//! The `model` service's cases.

use super::service::place;
use super::types::Model;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages::{Integer, Position};

/// What a row asks: the place of the element at this position, in a model whose elements are at places 0, 2 and 1.
type Input = Position;
/// The place the model answers.
type Value = i64;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_element_is_at_the_place_the_model_worked_out"),
        word: Word::Succeeds,
        input: || Position(1),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("the_first_element_of_a_container_is_at_place_zero"),
        word: Word::Succeeds,
        input: || Position(0),
        expect: Expect::Value(0),
    },
];

/// The place the model gives the row's position, over a model of three places.
fn subject(at: Input) -> Result<Value, Failure> {
    let model = Model {
        places: vec![Integer(0), Integer(2), Integer(1)],
    };
    Ok(place(&model, at).0)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
