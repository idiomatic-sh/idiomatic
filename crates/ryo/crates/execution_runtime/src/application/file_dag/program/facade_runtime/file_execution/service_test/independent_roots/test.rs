//! Real semantic refusal at one compiled root cannot erase the independent root.
use super::super::fixtures::Root;
use super::fixtures::{Failing, Observed, Readiness, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{NodeId, Value as Cell};
use ryo_values::domain::records::types::ObservationIndex;
use std::borrow::Cow;

type Input = Failing;
type Value = Observed;
type Failure = String;

/// The one function node of the file.
const FUNCTION: &[Cell] = &[Cell::Node(NodeId(0))];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("failed_emit_and_failed_observation_do_not_erase_the_independent_root"),
        word: Word::Succeeds,
        input: || Failing::Emit,
        expect: Expect::Value(Observed {
            emit: Root::Failed(OperatingError::WrongOperation),
            observation: [(ObservationIndex(0), Root::complete(FUNCTION, 1))],
            readiness: Some(Readiness::Complete),
            published: true,
        }),
    },
    Case {
        name: CaseName("a_failed_observation_does_not_erase_the_independent_diagnostic_root"),
        word: Word::Succeeds,
        input: || Failing::Observation,
        expect: Expect::Value(Observed {
            emit: Root::complete(FUNCTION, 1),
            observation: [(
                ObservationIndex(0),
                Root::Failed(OperatingError::WrongOperation),
            )],
            readiness: Some(Readiness::Unknown(Cow::Borrowed(
                "the operation is not this operator's",
            ))),
            published: false,
        }),
    },
];

/// Runs the producer and its consumer on one file with the refused root.
fn subject(failing: Input) -> Result<Value, Failure> {
    Ok(exercise(failing))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
