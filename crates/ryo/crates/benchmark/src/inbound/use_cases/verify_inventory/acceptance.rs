//! The public inventory use case preserves complete coverage and refuses invalid defaults.
#![expect(
    clippy::expect_used,
    reason = "acceptance fixtures fail on unexpected outcomes"
)]
use cucumber::then;
use ryo_benchmark::{
    domain::suite::types::inventory::value::InventoryVerdict,
    inbound::use_cases::verify_inventory::{
        interface::VerifyInventoryUseCase as _, use_case::VerifyInventoryUseCaseImpl,
    },
    outbound::{execution::fixtures::fake::ExecutionFake, inventory::factory::local},
};
use std::sync::Arc;

fn run(defaults: &[&str]) -> (Result<bool, String>, serde_json::Value, usize) {
    let root = tempfile::tempdir().expect("inventory fixture");
    let registration = root.path().join("registration.json");
    let output = root.path().join("result");
    std::fs::write(
        &registration,
        serde_json::json!({"authority": {"kind": "unavailable", "reason": "fixture registration has no implementation"}, "provider": "fixture", "defaults": defaults, "rules": []}).to_string(),
    )
    .expect("registration fixture");
    let execution = Arc::new(ExecutionFake::default());
    let operation = VerifyInventoryUseCaseImpl::new(
        Arc::new(ryo_benchmark::outbound::suite::factory::local()),
        Arc::new(ryo_benchmark::outbound::receipts::factory::local(
            ryo_benchmark::outbound::receipts::types::CaseReceipts::Unrecorded,
        )),
        execution.clone(),
        Arc::new(local()),
    );
    let result = operation
        .verify_inventory(&registration, &output)
        .map(|verdict| verdict == InventoryVerdict::Passed)
        .map_err(|error| error.to_string());
    let summary = if output.join("summary.json").is_file() {
        serde_json::from_slice(&std::fs::read(output.join("summary.json")).expect("summary"))
            .expect("summary JSON")
    } else {
        serde_json::Value::Null
    };
    let calls = execution.history.lock().expect("history").len();
    (result, summary, calls)
}

#[then("the inventory records a missing rule and completes")]
fn missing(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let (result, summary, calls) = run(&["missing"]);
    assert_eq!(result, Ok(false));
    assert_eq!(summary["run_state"], "complete");
    assert_eq!(summary["counts"], serde_json::json!({"PASS": 0, "FAIL": 1}));
    assert_eq!(
        summary["rules"][0]["reason"],
        "missing implementation registration"
    );
    assert_eq!(calls, 0);
}

#[then("the inventory refuses empty defaults before execution")]
fn empty(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let (result, summary, calls) = run(&[]);
    assert!(result.is_err());
    assert!(summary.is_null());
    assert_eq!(calls, 0);
}
