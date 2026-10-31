//! The worker publishes complete reports and never answers an engine refusal with success.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::outbound::{
    engine::fixtures::fake::EngineFake,
    worker::fixtures::{data::request, fake::WorkerFake},
};
use std::sync::Arc;
#[then("the worker preserves the checked file list")]
fn succeeds(world: &mut crate::Behavior) {
    let _ = world;
    let transport = Arc::new(WorkerFake::new(request()));
    RunRyoUseCaseImpl::new(Arc::new(EngineFake::default()), transport.clone())
        .run_ryo()
        .expect("worker report");
    assert_eq!(transport.reports()[0].files, request().files);
}
#[then("the worker preserves an engine refusal")]
fn refuses(world: &mut crate::Behavior) {
    let _ = world;
    let transport = Arc::new(WorkerFake::new(request()));
    let error = RunRyoUseCaseImpl::new(Arc::new(EngineFake { refuses: true }), transport.clone())
        .run_ryo()
        .expect_err("engine failure");
    assert_eq!(error.to_string(), "no candidate queries");
    assert_eq!(transport.respond_calls(), 0);
}
