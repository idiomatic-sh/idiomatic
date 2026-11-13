//! Executable acceptance steps for the derived view entry.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_derived::domain::contract::types::{
    BuildInputs, BuildOutcome, BuildWorkspace, DerivedView,
};
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::inbound::use_cases::build::interface::BuildUseCase as _;
use ryo_derived::inbound::use_cases::build::use_case::BuildUseCaseImpl;
use ryo_derived::inbound::use_cases::prepare_derived::interface::PrepareDerivedUseCase as _;
use ryo_derived::inbound::use_cases::prepare_derived::use_case::PrepareDerivedUseCaseImpl;
use ryo_derived::inbound::use_cases::view::interface::ViewUseCase as _;
use ryo_derived::inbound::use_cases::view::use_case::ViewUseCaseImpl;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

#[given(expr = "the view input {string}")]
fn given_input(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "view runs")]
fn runs(world: &mut Behavior) {
    let rows = Table {
        kinds: vec![CellKind::Number],
        cells: vec![Value::Number(Integer(4)), Value::Number(Integer(5))],
        count: RowCount(2),
    };
    let equality = PrepareDerivedUseCaseImpl::new().prepare_derived(
        DerivedRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        vec![vec![CellKind::Number]],
    );
    let raw = PrepareDerivedUseCaseImpl::new().prepare_derived(
        DerivedRequest::PartitionOrder {
            keys: vec![ColumnIndex(0)],
        },
        vec![vec![CellKind::Number]],
    );
    let mut room = BuildWorkspace::default();
    let built = if world.input == "partition order payload" {
        BuildUseCaseImpl::new().build(&raw, BuildInputs::Rows(&rows), &mut room, None)
    } else {
        BuildUseCaseImpl::new().build(&equality, BuildInputs::Rows(&rows), &mut room, None)
    };
    let Ok(BuildOutcome::Built(payload)) = built else {
        panic!("the payload must build");
    };
    world.answer = match ViewUseCaseImpl::new().view(&equality, &payload) {
        Ok(DerivedView::Equality(index)) => format!("index of {} rows", index.rows.len()),
        Ok(_) => panic!("an equality request views an index"),
        Err(DerivedFault::WrongOperation) => "wrong operation".to_owned(),
        Err(_) => panic!("view must answer an index or the wrong operation"),
    };
}

#[then(expr = "view answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber owns captured strings"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
