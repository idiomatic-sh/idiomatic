//! The operation's standard output is the worker's alone, and an absent reference tool is named.
use cucumber::then;
use ryo_benchmark::domain::operations::types::Operation;
use ryo_benchmark::inbound::use_cases::operate::{
    fixtures::{Scenario, run},
    types::OperationVerdict,
};

#[then("a parity operation writes exactly what its sealed worker wrote")]
fn parity(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let observed = run(Scenario {
        operation: Operation::Parity,
        tool_found: true,
        verdict: OperationVerdict::Passed,
    });
    assert_eq!(observed.answer, Ok(OperationVerdict::Passed));
    assert_eq!(observed.stdout, b"{\"metric\":{}}\n");
}

#[then("an operation without its reference tool is refused by the tool's name")]
fn absent(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let observed = run(Scenario {
        operation: Operation::Measure,
        tool_found: false,
        verdict: OperationVerdict::Passed,
    });
    assert!(
        observed.answer.as_ref().is_err_and(|error| error.contains("lint")),
        "{:?}",
        observed.answer
    );
    assert!(observed.calls.is_empty());
}
