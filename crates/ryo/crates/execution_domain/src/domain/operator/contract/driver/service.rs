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
    let refusal = match answer {
        Err(Fault::MissingCapture) if latched.is_none() => missing_capture(operation),
        Err(Fault::UnsupportedRender) if latched.is_none() => {
            Some(unsupported_render(driver.rendered_answers()))
        }
        _ => None,
    };
    let lease = driver.finish();
    lease.workspace.evaluation = scratch;
    if let Some(refusal) = refusal {
        return Err(refusal);
    }
    crate::domain::operating::service::written_result(answer, latched)
}

/// A named capture the matcher does not have is an invalid regular expression of the operation.
pub(super) fn missing_capture(operation: &Operation) -> Option<OperatingError> {
    let Operation::SourceMatches { capture, .. } = operation else {
        return None;
    };
    Some(OperatingError::InvalidRegex {
        reason: format!(
            "missing named capture `{}`",
            capture.as_ref().map(|name| &*name.0).unwrap_or_default()
        ),
    })
}

/// A provider's unsupported answer invalidates the consuming rule, with the provider's reason.
pub(super) fn unsupported_render(
    answers: &[ryo_language::domain::semantics::types::RenderOutcome],
) -> OperatingError {
    use ryo_language::domain::semantics::types::RenderOutcome;
    let reason = answers.iter().find_map(|answer| match answer {
        RenderOutcome::Unsupported(reason) => Some(reason.0),
        RenderOutcome::Known(_) | RenderOutcome::NotApplicable => None,
    });
    OperatingError::SemanticIncomplete {
        reason: format!(
            "selected language cannot render this syntax (reason {})",
            reason.map_or_else(String::new, |reason| reason.to_string())
        ),
    }
}
