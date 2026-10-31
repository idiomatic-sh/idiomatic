//! Worker acceptance checks delegation, independently from any concrete language provider.
use cucumber::then;
use ryo_benchmark::domain::suite::types::WorkerRequest;
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::outbound::engine::fixtures::fake::EngineFake;
use std::sync::Arc;
fn request() -> WorkerRequest {
    WorkerRequest {
        root: ".".into(),
        rules: "rules".into(),
        files: vec!["input.txt".into()],
        cache: None,
    }
}
#[then("the worker preserves the checked file list")]
fn succeeds(world: &mut crate::Behavior) {
    let _ = world;
    let worker = RunRyoUseCaseImpl::new(Arc::new(EngineFake::default()));
    let report = worker.run_ryo(request()).expect("worker report");
    assert_eq!(report.files, ["input.txt"]);
}
#[then("the worker preserves an engine refusal")]
fn refuses(world: &mut crate::Behavior) {
    let _ = world;
    let worker = RunRyoUseCaseImpl::new(Arc::new(EngineFake { refuses: true }));
    let error = worker.run_ryo(request()).expect_err("engine failure");
    assert_eq!(error.to_string(), "no candidate queries");
}
