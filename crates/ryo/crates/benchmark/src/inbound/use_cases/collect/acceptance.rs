//! The public collector operation preserves operational refusals and validates before launch.
#![expect(
    clippy::expect_used,
    reason = "acceptance assertions must fail the scenario on unexpected success or refusal"
)]
use cucumber::then;
use ryo_benchmark::inbound::use_cases::collect::interface::CollectUseCase as _;
use ryo_benchmark::inbound::use_cases::collect::use_case::CollectUseCaseImpl;
use ryo_benchmark::outbound::collection::fixtures::fake::CollectionFake;
use ryo_benchmark::outbound::collection::interface::CollectionPort as _;
use ryo_benchmark::outbound::collection::types::CollectorMessage;
use ryo_benchmark::outbound::execution::fixtures::fake::ExecutionFake;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::sync::Arc;
/// Assert that the collector preserves a missing executable error.
#[then("the collector preserves a missing executable error")]
fn succeeds(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let codec = Arc::new(CollectionFake::default());
    let request = codec
        .request(&Invocation {
            executable: "/missing/tool".into(),
            arguments: Vec::new(),
            environment: std::collections::BTreeMap::default(),
            directory: "/tmp".into(),
        })
        .expect("request");
    let operation = CollectUseCaseImpl::new(
        Arc::<CollectionFake>::clone(&codec),
        Arc::new(ExecutionFake::default()),
    );
    let response = operation
        .collect(&request)
        .expect("encoded operational failure");
    assert!(
        matches!(codec.observation(&response).expect("protocol"),Err(ReportsError::Io(error)) if error.kind()==std::io::ErrorKind::NotFound),
        "missing executable must remain an operational launch failure"
    );
}
/// Assert that the collector refuses malformed input before execution.
#[then("the collector refuses malformed input before execution")]
fn refuses(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let execution = Arc::new(ExecutionFake::default());
    let operation = CollectUseCaseImpl::new(
        Arc::new(CollectionFake::default()),
        Arc::<ExecutionFake>::clone(&execution),
    );
    assert!(
        matches!(operation.collect(&CollectorMessage(b"malformed".to_vec())),Err(ReportsError::Invalid(message)) if message=="malformed request"),
        "malformed transport must be refused before launch"
    );
    assert!(
        execution.history.lock().expect("history").is_empty(),
        "malformed input must not launch any contestant"
    );
}
