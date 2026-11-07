//! What the `build` use case promises, against the fakes of the ports it calls, from its fixtures.

use super::interface::BuildUseCase;
use super::use_case::BuildUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::{BuildInputs, BuildOutcome, BuildWorkspace};
use crate::domain::fault::types::DerivedFault;
use crate::domain::raw_words::types::RawWords;
use crate::domain::native::types::NativeForm;
use crate::domain::preparation::service::prepare;
use crate::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    EqualityBuilds,
    UnsupportedNativeDeclines,
    InputsOfAnotherKind,
}
type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_equality_request_builds_a_payload"),
        word: Word::Succeeds,
        input: || Scenario::EqualityBuilds,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_unsupported_native_shape_declines"),
        word: Word::Succeeds,
        input: || Scenario::UnsupportedNativeDeclines,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("inputs_of_another_kind_are_the_wrong_operation"),
        word: Word::Refuses,
        input: || Scenario::InputsOfAnotherKind,
        expect: Expect::Error(DerivedFault::WrongOperation),
    },
];

/// Calls the use case on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let rows = Table {
        kinds: vec![CellKind::Number],
        cells: [4, 5].map(|cell| Cell::Number(Integer(cell))).to_vec(),
        count: RowCount(2),
    };
    let schema = vec![CellKind::Number];
    let mut room = BuildWorkspace::default();
    let outcome = match input {
        Scenario::EqualityBuilds => BuildUseCaseImpl::new().build(
            &prepare(
                DerivedRequest::Equality { columns: vec![ColumnIndex(0)] },
                vec![schema],
            ),
            BuildInputs::Rows(&rows),
            &mut room,
            None,
        )?,
        Scenario::UnsupportedNativeDeclines => BuildUseCaseImpl::new().build(
            &prepare(
                DerivedRequest::NativeLookup {
                    left_on: vec![ColumnIndex(0), ColumnIndex(1)],
                    right_on: vec![ColumnIndex(0), ColumnIndex(1)],
                    view: Vec::new(),
                },
                vec![vec![CellKind::Number, CellKind::Number]; 2],
            ),
            BuildInputs::Native(NativeForm::Edges),
            &mut room,
            None,
        )?,
        Scenario::InputsOfAnotherKind => BuildUseCaseImpl::new().build(
            &prepare(
                DerivedRequest::Equality { columns: vec![ColumnIndex(0)] },
                vec![schema],
            ),
            BuildInputs::RawWords(RawWords(&[])),
            &mut room,
            None,
        )?,
    };
    Ok(usize::from(matches!(outcome, BuildOutcome::Built(_))))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
