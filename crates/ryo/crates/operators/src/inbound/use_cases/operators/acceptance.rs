//! Executable acceptance steps for the borrowed operator family entry.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase as _;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[given(expr = "the operators input {string}")]
fn given_input(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "operators runs")]
fn runs(world: &mut Behavior) {
    let row = [Value::Number(Integer(7)), Value::Number(Integer(9))];
    let columns: &[ColumnIndex] = if world.input == "missing column" {
        &[ColumnIndex(3), ColumnIndex(2)]
    } else {
        &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)]
    };
    let mut output = Vec::new();
    world.answer = match OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())).operators(OperatorCall::Project {
        row: &row,
        columns,
        target: &mut output,
    }) {
        Ok(OperatorAnswer::Written) => output
            .iter()
            .map(|value| match value {
                Value::Number(number) => number.0.to_string(),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>()
            .join(" "),
        Err(PortableOperatorFault::NoSuchColumn { index: missing }) => format!("missing {}", missing.0),
        _ => panic!("project must answer a written row or a missing column"),
    };
}

#[then(expr = "operators answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber owns captured strings"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
