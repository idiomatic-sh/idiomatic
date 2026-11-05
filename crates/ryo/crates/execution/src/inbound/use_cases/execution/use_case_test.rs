//! Public preparation must reject structurally mismatched request inputs.
use super::{interface::ExecutionUseCase, types::*, use_case::ExecutionUseCaseImpl};
use std::sync::Arc;
#[test]
fn rejects_binding_population_without_a_rule() {
    let engine = ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    let request = PreparedRequest {
        plans: vec![],
        bindings: vec![vec![]],
        rules: vec![],
        digests: vec![],
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    };
    assert!(matches!(
        engine.prepare(request),
        Err(OperatingError::ObservationConfiguration(_))
    ));
}
#[test]
fn empty_request_can_begin_and_finish_without_acquiring_inputs() {
    let engine = ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    let request = PreparedRequest {
        plans: vec![],
        bindings: vec![],
        rules: vec![],
        digests: vec![],
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    };
    let prepared = engine
        .prepare(request)
        .expect("empty request is legitimate");
    let run = engine.begin(&prepared, &[]);
    super::interface::ExecutionRequest::finish(run);
}
