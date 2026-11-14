//! The `driver` service: one evaluation through the operators' meaning of its kind, with the engine's
//! [`LeaseDriver`] behind their driver ports.

use super::super::primitive::types::PreparedInputs;
use super::super::types::{PrimitiveLease, RowSink};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::types::lease_driver::value::LeaseDriver;
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::evaluation::types::{EvaluationInput, EvaluationViews};
use ryo_values::domain::plans::types::Operation;

/// Evaluates one operation through the operators' one meaning of its kind: the engine lends its views, its
/// scratch and this driver, and keeps the first engine error ahead of any portable answer.
pub(crate) fn evaluate(
    operation: &Operation,
    inputs: PreparedInputs<'_>,
    lease: &mut PrimitiveLease<'_>,
    sink: &mut dyn RowSink,
) -> Result<(), OperatingError> {
    let semantics = crate::domain::operator::contract::lease::service::semantics(lease);
    let resources = lease.views.resources;
    let views = EvaluationViews {
        source: ryo_operators::domain::source_matches::types::LineBytes(lease.views.source.0),
        bindings: lease.views.bindings,
        regex: lease.views.regex.into(),
        checked: lease
            .views
            .validated_source
            .map(
                |checked| ryo_operators::domain::source_matches::types::CheckedSource {
                    base: checked.base,
                    text: checked.text,
                    line: checked.line,
                },
            )
            .into(),
        appended: resources.appended().into(),
        layers: resources.rank_layers().into(),
        cached_reach: lease
            .views
            .cached_reach
            .map(
                |reach| ryo_operators::domain::equality_join::types::CachedReach {
                    request: reach.request,
                    destinations: reach.destinations,
                },
            )
            .into(),
        cached_axis: lease.views.cached_axis.map(|axis| axis.destinations).into(),
    };
    let input = match inputs {
        PreparedInputs::Row { row, .. } => EvaluationInput::Row(row),
        PreparedInputs::Views(_) | PreparedInputs::RequestSource(_) => EvaluationInput::Populations,
    };
    let mut scratch = std::mem::take(&mut lease.workspace.evaluation);
    let mut driver = LeaseDriver::new(lease, sink, operation, inputs);
    let answer = semantics.operators(
        ryo_operators::inbound::use_cases::operators::types::OperatorCall::Evaluate {
            operation,
            input,
            views,
            driver: &mut driver,
            scratch: &mut scratch,
        },
    );
    let latched = driver.take_failure();
    let refusal = match &answer {
        Err(fault @ Fault::MissingCapture { .. }) if latched.is_none() => Some(
            crate::domain::operating::service::portable_fault(fault.clone()),
        ),
        Err(Fault::UnsupportedRender) if latched.is_none() => Some(
            crate::domain::operating::service::unsupported_render(driver.rendered_answers()),
        ),
        _ => None,
    };
    let lease = driver.finish();
    lease.workspace.evaluation = scratch;
    if let Some(refusal) = refusal {
        return Err(refusal);
    }
    crate::domain::operating::service::written_result(answer, latched)
}
