//! Executable acceptance steps for the derived build entry.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_derived::domain::contract::types::{BuildInputs, BuildOutcome, BuildWorkspace};
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::raw_words::types::RawWords;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::inbound::use_cases::build::interface::BuildUseCase as _;
use ryo_derived::inbound::use_cases::build::use_case::BuildUseCaseImpl;
use ryo_derived::inbound::use_cases::prepare_derived::interface::PrepareDerivedUseCase as _;
use ryo_derived::inbound::use_cases::prepare_derived::use_case::PrepareDerivedUseCaseImpl;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

#[given(expr = "the build input {string}")]
fn given_input(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "build runs")]
fn runs(world: &mut Behavior) {
    let rows = Table {
        kinds: vec![CellKind::Number],
        cells: vec![Value::Number(Integer(4)), Value::Number(Integer(5))],
        count: RowCount(2),
    };
    let prepared = PrepareDerivedUseCaseImpl::new().prepare(
        DerivedRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        vec![vec![CellKind::Number]],
    );
    let inputs = if world.input == "raw words" {
        BuildInputs::RawWords(RawWords(&[]))
    } else {
        BuildInputs::Rows(&rows)
    };
    let mut room = BuildWorkspace::default();
    world.answer = match BuildUseCaseImpl::new().build(&prepared, inputs, &mut room, None) {
        Ok(BuildOutcome::Built(_)) => "built".to_owned(),
        Ok(BuildOutcome::Declined) => "declined".to_owned(),
        Err(DerivedFault::WrongOperation) => "wrong operation".to_owned(),
        Err(_) => panic!("build must answer a payload or the wrong operation"),
    };
}

#[then(expr = "build answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber owns captured strings"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
