//! Executable acceptance steps for the derived preparation entry.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_derived::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[given(expr = "the prepare input {string}")]
fn given_input(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "prepare runs")]
fn runs(world: &mut Behavior) {
    let column = if world.input == "absent column" {
        ColumnIndex(3)
    } else {
        ColumnIndex(0)
    };
    let prepared = PrepareUseCaseImpl::new().prepare(
        DerivedRequest::Equality {
            columns: vec![column],
        },
        vec![vec![CellKind::Number]],
    );
    world.answer = match prepared.validate() {
        Ok(()) => format!("prepared {}", prepared.schemas().len()),
        Err(DerivedFault::NoSuchColumn { column }) => format!("no such column {}", column.0),
        Err(_) => panic!("prepare must answer a schema or a missing column"),
    };
}

#[then(expr = "prepare answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber owns captured strings"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
