//! Logical set lifetimes keep bag scopes and deferred errors explicit.
use super::types::WorkspaceLifetime as Lifetime;
use super::workspace::begin;
use crate::domain::{
    operating::errors::OperatingError,
    operator::contract::types::{PrimitiveWorkspace, WorkspaceProgram},
    table_dedup::service::first_time,
    table_model::types::Addition,
};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
#[derive(Clone, Copy)]
enum Scenario {
    Invocation,
    PrimaryRow,
    Unprepared,
    DeferredError,
}
type Input = Scenario;
type Value = [Addition; 3];
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("invocation_set_survives_rows_and_resets_for_retry"),
        word: Word::Succeeds,
        input: || Scenario::Invocation,
        expect: Expect::Value([Addition::Added, Addition::Existing, Addition::Added]),
    },
    Case {
        name: CaseName("reach_set_restarts_for_each_primary_row"),
        word: Word::Succeeds,
        input: || Scenario::PrimaryRow,
        expect: Expect::Value([Addition::Added, Addition::Added, Addition::Added]),
    },
    Case {
        name: CaseName("unprepared_logical_set_fails_closed"),
        word: Word::Refuses,
        input: || Scenario::Unprepared,
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        name: CaseName("unused_schema_fault_remains_pending"),
        word: Word::Refuses,
        input: || Scenario::DeferredError,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    if matches!(scenario, Scenario::Unprepared | Scenario::DeferredError) {
        let mut workspace = PrimitiveWorkspace::default();
        if matches!(scenario, Scenario::DeferredError) {
            begin(
                &WorkspaceProgram::FirstOccurrences {
                    lifetime: Lifetime::Invocation,
                    kinds: Err(OperatingError::ShapeMismatch),
                },
                Lifetime::Invocation,
                &mut workspace,
            );
        }
        let state = workspace
            .distinct
            .as_ref()
            .ok_or(OperatingError::WrongBinding)?;
        return state
            .schema
            .clone()
            .map(|()| [Addition::Added, Addition::Added, Addition::Added]);
    }
    let lifetime = match scenario {
        Scenario::Invocation => Lifetime::Invocation,
        Scenario::PrimaryRow => Lifetime::PrimaryRow,
        _ => return Err(OperatingError::WrongOperation),
    };
    let program = WorkspaceProgram::FirstOccurrences {
        lifetime,
        kinds: Ok(vec![CellKind::Number]),
    };
    let mut workspace = PrimitiveWorkspace::default();
    let mut found = Vec::new();
    begin(&program, lifetime, &mut workspace);
    let state = workspace
        .distinct
        .as_mut()
        .ok_or(OperatingError::WrongBinding)?;
    state.schema.clone()?;
    found.push(first_time(
        &mut state.rows,
        &mut state.seen,
        &[Cell::Number(Integer(1))],
    ));
    begin(&program, Lifetime::PrimaryRow, &mut workspace);
    let state = workspace
        .distinct
        .as_mut()
        .ok_or(OperatingError::WrongBinding)?;
    found.push(first_time(
        &mut state.rows,
        &mut state.seen,
        &[Cell::Number(Integer(1))],
    ));
    begin(&program, lifetime, &mut workspace);
    let state = workspace
        .distinct
        .as_mut()
        .ok_or(OperatingError::WrongBinding)?;
    found.push(first_time(
        &mut state.rows,
        &mut state.seen,
        &[Cell::Number(Integer(1))],
    ));
    found.try_into().map_err(|_| OperatingError::ShapeMismatch)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
