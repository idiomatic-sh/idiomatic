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

/// How far an equality request is taken across the port.
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    /// Prepared and validated.
    Prepared,
    /// Built from the two rows.
    BuiltFromRows,
    /// Built from native edges, another kind of input than the request's.
    BuiltFromEdges,
    /// Built from the two rows, then viewed.
    Viewed,
}

/// An equality request on `column`, taken to `stage`.
#[derive(Clone, Copy, Debug)]
pub struct Request {
    /// The column the request looks up.
    pub column: ColumnIndex,
    /// How far the request is taken.
    pub stage: Stage,
}

/// What the stage answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The number of schemas the prepared request keeps.
    Schemas(usize),
    /// Whether the build produced a payload.
    Built(bool),
    /// The number of rows the viewed index holds.
    IndexRows(usize),
}

type Input = Request;
type Value = Answer;
type Failure = DerivedFault;

/// An equality request on column `column`, taken to `stage`.
const fn request(column: u32, stage: Stage) -> Request {
    Request {
        column: ColumnIndex(column),
        stage,
    }
}

/// Observable schema, build and view of an equality request, and the refusals across the port.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_request_on_a_present_column_keeps_its_schema"),
        word: Word::Succeeds,
        input: || request(0, Stage::Prepared),
        expect: Expect::Value(Answer::Schemas(1)),
    },
    Case {
        name: CaseName("an_equality_request_builds_a_payload"),
        word: Word::Succeeds,
        input: || request(0, Stage::BuiltFromRows),
        expect: Expect::Value(Answer::Built(true)),
    },
    Case {
        name: CaseName("an_equality_payload_is_viewed_as_its_index"),
        word: Word::Succeeds,
        input: || request(0, Stage::Viewed),
        expect: Expect::Value(Answer::IndexRows(2)),
    },
    Case {
        name: CaseName("a_request_on_an_absent_column_is_refused_on_validation"),
        word: Word::Refuses,
        input: || request(3, Stage::Prepared),
        expect: Expect::Error(DerivedFault::NoSuchColumn {
            column: ColumnIndex(3),
        }),
    },
    Case {
        name: CaseName("inputs_of_another_kind_are_the_wrong_operation"),
        word: Word::Refuses,
        input: || request(0, Stage::BuiltFromEdges),
        expect: Expect::Error(DerivedFault::WrongOperation),
    },
];

/// Run the same observable cases for every implementation of the engine port.
///
/// # Panics
///
/// When the implementation changes the schema it keeps, what it builds or views, or its refusals.
pub fn check(adapter: &impl DerivedPort) {
    let subject = |request: Input| {
        let rows = Table {
            kinds: vec![CellKind::Number],
            cells: [4, 5].map(|cell| Cell::Number(Integer(cell))).to_vec(),
            count: RowCount(2),
        };
        let prepared = adapter.prepare(
            DerivedRequest::Equality {
                columns: vec![request.column],
            },
            vec![vec![CellKind::Number]].into(),
        );
        prepared.validate()?;
        let mut room = BuildWorkspace::default();
        match request.stage {
            Stage::Prepared => Ok(Answer::Schemas(prepared.schemas().len())),
            Stage::BuiltFromRows => {
                let built =
                    adapter.build(&prepared, BuildInputs::Rows(&rows), &mut room, None.into())?;
                Ok(Answer::Built(matches!(built, BuildOutcome::Built(_))))
            }
            Stage::Viewed => {
                let BuildOutcome::Built(payload) =
                    adapter.build(&prepared, BuildInputs::Rows(&rows), &mut room, None.into())?
                else {
                    return Err(DerivedFault::NotRun);
                };
                let DerivedView::Equality(index) = adapter.view(&prepared, &payload)? else {
                    return Err(DerivedFault::WrongOperation);
                };
                Ok(Answer::IndexRows(index.rows.len()))
            }
            Stage::BuiltFromEdges => {
                let built = adapter.build(
                    &prepared,
                    BuildInputs::Native(ryo_derived::domain::native::types::NativeForm::Edges),
                    &mut room,
                    None.into(),
                )?;
                Ok(Answer::Built(matches!(built, BuildOutcome::Built(_))))
            }
        }
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
