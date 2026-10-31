//! The public collector operation preserves operational refusals and validates before launch.
use cucumber::then;
use ryo_benchmark::{
    domain::{diagnostics::errors::DiagnosticsError, measurements::types::Invocation},
    inbound::use_cases::collect::{interface::CollectUseCase, use_case::CollectUseCaseImpl},
    outbound::{
        collection::{
            fixtures::fake::CollectionFake, interface::CollectionPort, types::CollectorMessage,
        },
        execution::fixtures::fake::ExecutionFake,
    },
};
use std::sync::Arc;
#[then("the collector preserves a missing executable error")]
fn succeeds(world: &mut crate::Behavior) {
    let _ = world;
    let codec = Arc::new(CollectionFake::default());
    let request = codec
        .request(&Invocation {
            executable: "/missing/tool".into(),
            arguments: Vec::new(),
            environment: Default::default(),
            directory: "/tmp".into(),
        })
        .expect("request");
    let operation = CollectUseCaseImpl::new(codec.clone(), Arc::new(ExecutionFake::default()));
    let response = operation
        .collect(&request)
        .expect("encoded operational failure");
    assert!(
        matches!(codec.observation(&response).expect("protocol"),Err(DiagnosticsError::Io(error)) if error.kind()==std::io::ErrorKind::NotFound)
    );
}
#[then("the collector refuses malformed input before execution")]
fn refuses(world: &mut crate::Behavior) {
    let _ = world;
    let execution = Arc::new(ExecutionFake::default());
    let operation = CollectUseCaseImpl::new(Arc::new(CollectionFake::default()), execution.clone());
    assert!(
        matches!(operation.collect(&CollectorMessage(b"malformed".to_vec())),Err(DiagnosticsError::Invalid(message)) if message=="malformed request")
    );
    assert!(execution.history.lock().expect("history").is_empty());
}
