//! Bound input admission proved with typed declarations, without parser or port setup.

use super::errors::ObservationInputsError;
use super::service::validate_inputs;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::hosted::types::{HostedName, HostedRelation};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
};
use ryo_values::domain::plans::types::StepScope;

#[derive(Clone, Copy)]
enum Scenario {
    Empty,
    Distinct,
    CatalogNameNotAdmitted,
    BlankName,
    DuplicateInput,
    CatalogCollision,
    HostedCollision,
    BlankColumn,
    DuplicateColumn,
}
type Input = Scenario;
type Value = ();
type Failure = ObservationInputsError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_bound_inputs"),
        word: Word::Succeeds,
        input: || Input::Empty,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("column_names_are_local_to_each_input"),
        word: Word::Succeeds,
        input: || Input::Distinct,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("catalog_names_only_collide_when_admitted"),
        word: Word::Succeeds,
        input: || Input::CatalogNameNotAdmitted,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("blank_input"),
        word: Word::Refuses,
        input: || Input::BlankName,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ObservationInputsError::BlankName))
        }),
    },
    Case {
        name: CaseName("duplicate_input"),
        word: Word::Refuses,
        input: || Input::DuplicateInput,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ObservationInputsError::SourceCollision { name } if name.0.as_ref() == "observed"),
            )
        }),
    },
    Case {
        name: CaseName("catalog_collision"),
        word: Word::Refuses,
        input: || Input::CatalogCollision,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ObservationInputsError::SourceCollision { name } if name.0.as_ref() == "function"),
            )
        }),
    },
    Case {
        name: CaseName("hosted_collision"),
        word: Word::Refuses,
        input: || Input::HostedCollision,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ObservationInputsError::SourceCollision { name } if name.0.as_ref() == "host.placed"),
            )
        }),
    },
    Case {
        name: CaseName("blank_column"),
        word: Word::Refuses,
        input: || Input::BlankColumn,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ObservationInputsError::Column { input, column } if input.0.as_ref() == "observed" && column.0.as_ref() == " "),
            )
        }),
    },
    Case {
        name: CaseName("duplicate_column"),
        word: Word::Refuses,
        input: || Input::DuplicateColumn,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ObservationInputsError::Column { input, column } if input.0.as_ref() == "observed" && column.0.as_ref() == "id"),
            )
        }),
    },
];

fn declaration(name: &str, columns: &[&str]) -> BoundObservationInput {
    BoundObservationInput {
        name: ObservationBindingName(name.into()),
        columns: columns
            .iter()
            .map(|name| ObservationColumn {
                name: ObservationColumnName((*name).into()),
                kind: CellKind::Node,
            })
            .collect(),
        scope: StepScope::PerFile,
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    let mut catalog = Vec::new();
    let mut hosted = Vec::new();
    let inputs = match input {
        Input::Empty => Vec::new(),
        Input::Distinct => vec![
            declaration("first", &["id"]),
            declaration("second", &["id"]),
        ],
        Input::CatalogNameNotAdmitted => vec![declaration("function", &["id"])],
        Input::BlankName => vec![declaration(" \t", &["id"])],
        Input::DuplicateInput => vec![
            declaration("observed", &["id"]),
            declaration("observed", &["other"]),
        ],
        Input::CatalogCollision => {
            catalog.push(Relation::Function);
            vec![declaration("function", &["id"])]
        }
        Input::HostedCollision => {
            hosted.push(HostedRelation {
                name: HostedName("host.placed".into()),
                columns: Vec::new(),
            });
            vec![declaration("host.placed", &["id"])]
        }
        Input::BlankColumn => vec![declaration("observed", &[" "])],
        Input::DuplicateColumn => vec![declaration("observed", &["id", "id"])],
    };
    validate_inputs(&catalog, &hosted, &inputs)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
