//! The `dispatch` service's cases, on values alone.

use super::service::{build, native_finding, view};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::{BuildInputs, BuildOutcome, BuildWorkspace, DerivedView};
use crate::domain::fault::types::DerivedFault;
use crate::domain::native::types::NativeForm;
use crate::domain::preparation::service::prepare;
use crate::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    EqualityBuildsAndViews,
    NativeShapeDeclinesWhenUnsupported,
    InputsOfAnotherKindAreRefused,
}
type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_equality_build_is_viewed_as_its_index"),
        word: Word::Succeeds,
        input: || Scenario::EqualityBuildsAndViews,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("an_unsupported_native_shape_declines"),
        word: Word::Succeeds,
        input: || Scenario::NativeShapeDeclinesWhenUnsupported,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("inputs_of_another_kind_are_the_wrong_operation"),
        word: Word::Refuses,
        input: || Scenario::InputsOfAnotherKindAreRefused,
        expect: Expect::Error(DerivedFault::WrongOperation),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let rows = Table {
        kinds: vec![CellKind::Number],
        cells: [4, 5].map(|cell| Cell::Number(Integer(cell))).to_vec(),
        count: RowCount(2),
    };
    let schemas = vec![vec![CellKind::Number]];
    let equality = prepare(
        DerivedRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        schemas.clone(),
    );
    let mut room = BuildWorkspace::default();
    match input {
        Scenario::EqualityBuildsAndViews => {
            let BuildOutcome::Built(payload) =
                build(&equality, BuildInputs::Rows(&rows), &mut room, None)?
            else {
                return Err(DerivedFault::NotRun);
            };
            let DerivedView::Equality(index) = view(&equality, &payload)? else {
                return Err(DerivedFault::WrongOperation);
            };
            Ok(index.rows.len())
        }
        Scenario::NativeShapeDeclinesWhenUnsupported => {
            let native = prepare(
                DerivedRequest::NativeLookup {
                    left_on: vec![ColumnIndex(0), ColumnIndex(1)],
                    right_on: vec![ColumnIndex(0), ColumnIndex(1)],
                    view: Vec::new(),
                },
                vec![vec![CellKind::Number, CellKind::Number]; 2],
            );
            Ok(usize::from(
                native_finding(&native, NativeForm::Edges)?.is_some(),
            ))
        }
        Scenario::InputsOfAnotherKindAreRefused => build(
            &equality,
            BuildInputs::Native(NativeForm::Edges),
            &mut room,
            None,
        )
        .map(|_| 0),
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
