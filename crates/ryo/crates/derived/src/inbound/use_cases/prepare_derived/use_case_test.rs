//! What the `prepare_derived` use case promises, against the fakes of the ports it calls, from its fixtures.

use super::interface::PrepareDerivedUseCase;
use super::use_case::PrepareDerivedUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use crate::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    PresentColumn,
    AbsentColumn,
}
type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_request_on_a_present_column_keeps_its_schema"),
        word: Word::Succeeds,
        input: || Scenario::PresentColumn,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_request_on_an_absent_column_is_refused_on_validation"),
        word: Word::Refuses,
        input: || Scenario::AbsentColumn,
        expect: Expect::Error(DerivedFault::NoSuchColumn {
            column: ColumnIndex(3),
        }),
    },
];

/// Calls the use case on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let column = match input {
        Scenario::PresentColumn => ColumnIndex(0),
        Scenario::AbsentColumn => ColumnIndex(3),
    };
    let prepared = PrepareDerivedUseCaseImpl::new().prepare_derived(
        DerivedRequest::Equality {
            columns: vec![column],
        },
        vec![vec![CellKind::Number]],
    );
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
