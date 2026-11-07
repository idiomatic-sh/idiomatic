//! The worker publishes complete reports and never answers an engine refusal with success.
#![expect(
    clippy::expect_used,
    reason = "acceptance assertions must fail the scenario on unexpected success or refusal"
)]
use cucumber::then;
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::outbound::{
    context::fixtures::fake::ContextFake,
    engine::fixtures::fake::EngineFake,
    worker::fixtures::{data::request, fake::WorkerFake},
};
use std::sync::Arc;
/// Assert that the worker preserves the checked file list.
#[then("the worker preserves the checked file list")]
fn succeeds(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let transport = Arc::new(WorkerFake::new(request()));
    RunRyoUseCaseImpl::new(
        Arc::new(EngineFake::default()),
        Arc::<WorkerFake>::clone(&transport),
        Arc::new(ContextFake),
    )
    .run_ryo()
    .expect("worker report");
    assert_eq!(
        transport.reports().first().expect("published report").files,
        request().files,
        "worker must retain exact checked inputs"
    );
}
/// Assert that the worker preserves an engine refusal.
#[then("the worker preserves an engine refusal")]
fn refuses(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let transport = Arc::new(WorkerFake::new(request()));
    let error = RunRyoUseCaseImpl::new(
        Arc::new(EngineFake { refuses: true }),
        Arc::<WorkerFake>::clone(&transport),
        Arc::new(ContextFake),
    )
    .run_ryo()
    .expect_err("engine failure");
    assert_eq!(
        error.to_string(),
        "no candidate queries",
        "engine refusal must reach caller"
    );
    assert_eq!(
        transport.respond_calls(),
        0,
        "failed engine must not publish a report"
    );
}
