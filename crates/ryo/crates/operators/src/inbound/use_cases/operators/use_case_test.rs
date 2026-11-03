//! The family entry dispatches borrowed semantic requests without replacing scratch ownership.
use super::{interface::OperatorsUseCase as _, use_case::OperatorsUseCaseImpl};
use crate::domain::contract::types::{
    OperatorAnswer, OperatorCall, PortableOperatorFault, ProjectColumnAbsent,
};
use ryo_language::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    Project,
    Append,
    MissingProjectColumn,
}
type Input = Scenario;
type Value = [Cell; 3];
type Failure = PortableOperatorFault;
const ROW: [Cell; 2] = [Cell::Number(Integer(7)), Cell::Number(Integer(9))];
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("project_preserves_repeated_plan_column"),
        word: Word::Succeeds,
        input: || Scenario::Project,
        expect: Expect::Value([ROW[1], ROW[0], ROW[1]]),
    },
    Case {
        name: CaseName("append_preserves_row_and_suffix_order"),
        word: Word::Succeeds,
        input: || Scenario::Append,
        expect: Expect::Value([ROW[0], ROW[1], ROW[0]]),
    },
    Case {
        name: CaseName("project_reports_absent_column"),
        word: Word::Refuses,
        input: || Scenario::MissingProjectColumn,
        expect: Expect::Error(PortableOperatorFault::Project(ProjectColumnAbsent(2))),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let port = OperatorsUseCaseImpl::new();
    let mut output = Vec::with_capacity(8);
    let original = output.as_ptr();
    let call = match input {
        Scenario::Project => OperatorCall::Project {
            row: &ROW,
            columns: &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
            target: &mut output,
        },
        Scenario::Append => OperatorCall::Append {
            row: &ROW,
            appended: &[ROW[0]],
            target: &mut output,
        },
        Scenario::MissingProjectColumn => OperatorCall::Project {
            row: &ROW,
            columns: &[ColumnIndex(2)],
            target: &mut output,
        },
    };
    assert_eq!(port.operators(call)?, OperatorAnswer::Written);
    assert_eq!(
        output.as_ptr(),
        original,
        "borrowed output keeps its allocation"
    );
    Ok(output.try_into().expect("three values are produced"))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
