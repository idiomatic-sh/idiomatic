//! Measurement acceptance exercises the public operation with deterministic ports.
use cucumber::then;
use ryo_benchmark::domain::suite::types::Audit;
use ryo_benchmark::domain::suite::types::RunRequest;
use ryo_benchmark::inbound::use_cases::measure::service::measure;
use ryo_benchmark::outbound::{
    execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
};
fn request() -> RunRequest {
    RunRequest {
        root: ".".into(),
        cache: "cache".into(),
        files: vec!["input.txt".into()],
    }
}
#[then("matching reports release a measured pair")]
fn matching(world: &mut crate::Behavior) {
    let _ = world;
    let audit = Audit {
        implemented: vec!["fixture".into()],
        missing: Vec::new(),
    };
    let pair = measure(
        &audit,
        &LinterFake,
        &LinterFake,
        &ExecutionFake { status: 0 },
        &request(),
    )
    .expect("measured pair");
    assert_eq!(pair.reference.report, pair.candidate.report);
}
#[then("incomplete coverage releases no measured pair")]
fn incomplete(world: &mut crate::Behavior) {
    let _ = world;
    let audit = Audit {
        implemented: Vec::new(),
        missing: vec!["fixture".into()],
    };
    let error = measure(
        &audit,
        &LinterFake,
        &LinterFake,
        &ExecutionFake { status: 0 },
        &request(),
    )
    .expect_err("incomplete coverage");
    assert_eq!(
        error.to_string(),
        "default coverage incomplete: 1 missing rules: fixture"
    );
}
