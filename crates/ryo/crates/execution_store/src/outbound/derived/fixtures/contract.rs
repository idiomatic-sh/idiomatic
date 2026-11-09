//! Behavioral contract shared by the injected derived adapter and test fake.
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::{
    BuildInputs, BuildOutcome, BuildWorkspace, DerivedView,
};
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

/// What the contract asks of the port.
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    /// An equality request on a present column keeps its schema.
    PresentColumn,
    /// An equality request on an absent column is refused on validation.
    AbsentColumn,
    /// An equality request builds a payload from rows.
    Builds,
    /// A built equality payload is viewed as its index.
    ViewsIndex,
    /// Inputs of another kind than the request's are the wrong operation.
    BuildsFromAnotherKind,
}

type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// Observable schema, build and view of an equality request, and the refusals across the port.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_request_on_a_present_column_keeps_its_schema"),
        word: Word::Succeeds,
        input: || Scenario::PresentColumn,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_equality_request_builds_a_payload"),
        word: Word::Succeeds,
        input: || Scenario::Builds,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_equality_payload_is_viewed_as_its_index"),
        word: Word::Succeeds,
        input: || Scenario::ViewsIndex,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_request_on_an_absent_column_is_refused_on_validation"),
        word: Word::Refuses,
        input: || Scenario::AbsentColumn,
        expect: Expect::Error(DerivedFault::NoSuchColumn {
            column: ColumnIndex(3),
        }),
    },
    Case {
        name: CaseName("inputs_of_another_kind_are_the_wrong_operation"),
        word: Word::Refuses,
        input: || Scenario::BuildsFromAnotherKind,
        expect: Expect::Error(DerivedFault::WrongOperation),
    },
];

/// Run the same observable cases for every implementation of the engine port.
///
/// # Panics
///
/// When the implementation changes the schema it keeps, what it builds or views, or its refusals.
pub fn check(adapter: &impl DerivedPort) {
    let subject = |scenario: Input| {
        let rows = Table {
            kinds: vec![CellKind::Number],
            cells: [4, 5].map(|cell| Cell::Number(Integer(cell))).to_vec(),
            count: RowCount(2),
        };
        let column = match scenario {
            Scenario::AbsentColumn => ColumnIndex(3),
            _ => ColumnIndex(0),
        };
        let prepared = adapter.prepare(
            DerivedRequest::Equality {
                columns: vec![column],
            },
            vec![vec![CellKind::Number]],
        );
        prepared.validate()?;
        let mut room = BuildWorkspace::default();
        match scenario {
            Scenario::PresentColumn | Scenario::AbsentColumn => Ok(prepared.schemas().len()),
            Scenario::Builds => {
                let built = adapter.build(&prepared, BuildInputs::Rows(&rows), &mut room, None)?;
                Ok(usize::from(matches!(built, BuildOutcome::Built(_))))
            }
            Scenario::ViewsIndex => {
                let BuildOutcome::Built(payload) =
                    adapter.build(&prepared, BuildInputs::Rows(&rows), &mut room, None)?
                else {
                    return Err(DerivedFault::NotRun);
                };
                match adapter.view(&prepared, &payload)? {
                    DerivedView::Equality(index) => Ok(index.rows.len()),
                    _ => Err(DerivedFault::WrongOperation),
                }
            }
            Scenario::BuildsFromAnotherKind => {
                let built = adapter.build(
                    &prepared,
                    BuildInputs::Native(ryo_derived::domain::native::types::NativeForm::Edges),
                    &mut room,
                    None,
                )?;
                Ok(usize::from(matches!(built, BuildOutcome::Built(_))))
            }
        }
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
