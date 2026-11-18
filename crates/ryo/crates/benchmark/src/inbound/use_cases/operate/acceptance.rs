//! A bench's standard output is the worker's alone, a failed verification runs no corpus, and an absent reference tool is named.
use cucumber::then;

use ryo_benchmark::domain::operations::types::BenchOperation;
use ryo_benchmark::inbound::use_cases::operate::{
    fixtures::{Scenario, run},
    types::OperationVerdict,
};

/// A bench on four processors whose one default the profile defers, so its verification passes.
const PASSING: Scenario = Scenario {
    operation: BenchOperation::Bench,
    tool_found: true,
    verdict: OperationVerdict::Passed,
    stray_rule: false,
    deferred: true,
    processors: 4,
};

#[then(
    "a bench writes the deterministic measures of the document its sealed worker wrote, as one BMF document"
)]
fn bench(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let observed = run(PASSING);
    assert_eq!(observed.answer, Ok(OperationVerdict::Passed));
    assert_eq!(
        observed.stdout,
        b"{\"fixture/project\":{\"peak-rss-bytes\":{\"value\":4096.0},\"store-bytes\":{\"value\":512.0}}}\n"
    );
    assert!(
        serde_json::from_slice::<serde_json::Value>(&observed.stdout).is_ok(),
        "the bench's standard output is one BMF document"
    );
}

#[then("a bench whose verification fails runs no corpus")]
fn failed_inventory(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let observed = run(Scenario {
        deferred: false,
        ..PASSING
    });
    assert_eq!(observed.answer, Ok(OperationVerdict::Failed));
    assert!(observed.stdout.is_empty(), "no corpus ran");
    assert!(
        !observed.calls.iter().any(|call| matches!(
            call,
            ryo_benchmark::outbound::executables::fixtures::fake::ExecutablesCall::Run(..)
        )),
        "{:?}",
        observed.calls
    );
}

#[then("an operation without its reference tool is refused by the tool's name")]
fn absent(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let observed = run(Scenario {
        tool_found: false,
        ..PASSING
    });
    assert!(
        observed
            .answer
            .as_ref()
            .is_err_and(|error| error.contains("lint")),
        "{:?}",
        observed.answer
    );
    assert!(observed.calls.is_empty());
}
