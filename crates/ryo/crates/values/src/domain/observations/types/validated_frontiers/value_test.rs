//! A frontier map is lent as `ValidatedFrontiers` only when it has the shape the request names.
use super::value::ValidatedFrontiers;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::observations::errors::ObservationsError;
use crate::domain::observations::service::{new_frontiers, validate_frontiers};
use crate::domain::records::types::{ObservationReadinessCount, ObservationReadinessLayout};
use crate::domain::stored_facts::types::Digest;

/// The slots the map is made with, and the slots the request names.
type Input = (u32, u32);
/// The slots of the map the checked view lends.
type Value = u32;
type Failure = ObservationsError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a map of the requested slots is lent as the checked view"),
        word: Word::Succeeds,
        input: || (3, 3),
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a map of other slots is not lent"),
        word: Word::Refuses,
        input: || (3, 4),
        expect: Expect::Error(ObservationsError::Invalid),
    },
];
fn subject((made, requested): Input) -> Result<Value, Failure> {
    let layout = ObservationReadinessLayout(Digest([7; 16]));
    let map = new_frontiers(layout, ObservationReadinessCount(made));
    let checked: ValidatedFrontiers<'_> =
        validate_frontiers(&map, layout, ObservationReadinessCount(requested))?;
    Ok(checked.0.slots.0)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
