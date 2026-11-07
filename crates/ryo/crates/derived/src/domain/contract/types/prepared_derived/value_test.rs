//! Prepared requests answer what preparation stored.
use super::value::PreparedDerived;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use crate::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    ValidMetadata,
    StoredFault,
}
type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("valid_metadata_answers_the_schemas"),
        word: Word::Succeeds,
        input: || Scenario::ValidMetadata,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_stored_metadata_fault_is_returned_by_validation"),
        word: Word::Refuses,
        input: || Scenario::StoredFault,
        expect: Expect::Error(DerivedFault::ShapeMismatch),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let prepared = PreparedDerived {
        request: DerivedRequest::Membership { column: ColumnIndex(0) },
        schemas: vec![vec![CellKind::Number]],
        metadata: match input {
            Scenario::ValidMetadata => Ok(()),
            Scenario::StoredFault => Err(DerivedFault::ShapeMismatch),
        },
    };
    prepared.validate()?;
    Ok(prepared.schemas().len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
