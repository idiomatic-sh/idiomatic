//! The names, columns and schemas of observation inputs, from the compile inbound API.

use super::super::{Given, compile_twice, observed, text_the_fake_reads};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::compile::errors::CompileError;
use crate::selects;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::hosted::types::ColumnLabel;
use ryo_values::domain::hosted::types::HostedColumn;
use ryo_values::domain::hosted::types::HostedName;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::observations::types::ObservationBindingName;
use ryo_values::domain::observations::types::ObservationColumn;
use ryo_values::domain::observations::types::ObservationColumnName;
use ryo_values::domain::plan_parts::types::StepScope;

/// What a row gives the use case.
type Input = fn() -> Given;
/// What `compile` answers: how many steps the plan has.
type Value = usize;
type Failure = CompileError;

fn subject(given: Input) -> Result<Value, Failure> {
    compile_twice(given())
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_distinct_observation_schema_is_admitted"),
        word: Word::Succeeds,
        input: || {
            || Given {
                observations: vec![observed("external", &["file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("observation_cannot_shadow_catalog_relation"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("function", &["file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("duplicate_observation_sources_are_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![
                    observed("external", &["file"]),
                    observed("external", &["file"]),
                ],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("blank_observation_column_is_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("external", &[" "])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("duplicate_observation_columns_are_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("external", &["file", "file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("observation_cannot_shadow_hosted_relation"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("external", &["file"])],
                hosted: vec![HostedRelation {
                    name: HostedName("external".into()),
                    columns: vec![HostedColumn {
                        name: ColumnLabel("file".into()),
                        kind: CellKind::File,
                    }],
                }],
                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
