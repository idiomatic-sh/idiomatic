//! What the `view` use case promises, against the fakes of the ports it calls, from its fixtures.

use super::interface::ViewUseCase;
use super::use_case::ViewUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::{BuildInputs, BuildOutcome, BuildWorkspace, DerivedView};
use crate::domain::fault::types::DerivedFault;
use crate::domain::preparation::service::prepare;
use crate::domain::raw_words::types::{RawWord, KeyStride};
use crate::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    EqualityIsViewedAsItsIndex,
    PayloadOfAnotherKind,
}
type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_equality_payload_is_viewed_as_its_index"),
        word: Word::Succeeds,
        input: || Scenario::EqualityIsViewedAsItsIndex,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_payload_of_another_kind_is_the_wrong_operation"),
        word: Word::Refuses,
        input: || Scenario::PayloadOfAnotherKind,
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
    let equality = prepare(
        DerivedRequest::Equality { columns: vec![ColumnIndex(0)] },
        vec![vec![CellKind::Number]],
    );
    let raw = prepare(DerivedRequest::RawKeyOrder { stride: KeyStride(1) }, Vec::new());
    let mut room = BuildWorkspace::default();
    let built = match input {
        Scenario::EqualityIsViewedAsItsIndex => crate::domain::dispatch::service::build(
            &equality,
            BuildInputs::Rows(&rows),
            &mut room,
            None,
        )?,
        Scenario::PayloadOfAnotherKind => crate::domain::dispatch::service::build(
            &raw,
            BuildInputs::RawWords(&[RawWord(1)]),
            &mut room,
            None,
        )?,
    };
    let BuildOutcome::Built(payload) = built else {
        return Err(DerivedFault::NotRun);
    };
    match ViewUseCaseImpl::new().view(&equality, &payload)? {
        DerivedView::Equality(index) => Ok(index.rows.len()),
        _ => Err(DerivedFault::WrongOperation),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
