//! Correctness gates each measured sample, including cache and edited runs.
use crate::application::run::service::run;
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::diagnostics::service::compare_with_contract;
use crate::domain::diagnostics::types::ComparisonContract;
use crate::domain::measurements::types::{RunOrder, pair::value::Pair};
use crate::domain::suite::service::require_complete;
use crate::domain::suite::types::RunRequest;
use crate::outbound::{execution::interface::ExecutionPort, linter::interface::LinterPort};

/// Execute both contestants, requiring all diagnostic findings and valid candidate execution; report incomparable reference metadata.
pub(super) fn measure(
    audit: &Audit,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
    order: RunOrder,
) -> Result<Pair, Error> {
    require_complete(audit)?;
    let (reference, candidate) = match order {
        RunOrder::ReferenceFirst => (
            run(reference, execution, request)?,
            run(candidate, execution, request)?,
        ),
        RunOrder::CandidateFirst => {
            let candidate = run(candidate, execution, request)?;
            (run(reference, execution, request)?, candidate)
        }
    };
    let comparison = compare_with_contract(
        &reference.report,
        &candidate.report,
        ComparisonContract::Diagnostics,
    )
    .map_err(|difference| {
        Error::Invalid(format!(
            "diagnostic benchmark contract failed: {difference:?}"
        ))
    })?;
    crate::outbound::evidence::adapters::comparison::write(
        &reference.report,
        &candidate.report,
        comparison,
        ComparisonContract::Diagnostics,
    );
    Ok(Pair {
        eligibility: crate::domain::measurements::types::MeasurementEligibility::Diagnostic,
        reference,
        candidate,
    })
}

use crate::domain::suite::types::audit::value::Audit;

/// Measure only the exact case-certified subset and immutable source bytes.
pub(super) fn measure_certified(
    certificate: &crate::domain::suite::types::certificate::value::Certificate,
    mode: &crate::domain::measurements::types::MeasurementMode,
    suite: &dyn crate::outbound::suite::interface::SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
    order: RunOrder,
) -> Result<Pair, Error> {
    crate::domain::measurements::service::require_inventory(
        mode,
        certificate.coverage.authored(),
        request.selected_rules.as_deref(),
    )?;
    if execution.profile() != certificate.execution {
        return Err(Error::Invalid(
            super::constants::CHANGED_INSTRUMENTATION.into(),
        ));
    }
    if suite.fingerprint(crate::outbound::suite::types::ArtifactSet(
        &certificate.paths,
    ))? != certificate.fingerprint
    {
        return Err(Error::Invalid(super::constants::CHANGED_EVIDENCE.into()));
    }
    if [
        reference.prepare(&certificate.probe)?,
        candidate.prepare(&certificate.probe)?,
    ] != certificate.profiles
    {
        return Err(Error::Invalid(super::constants::CHANGED_COMMAND.into()));
    }
    for (tool, expected) in [
        (reference, &certificate.contestants[0]),
        (candidate, &certificate.contestants[1]),
    ] {
        if tool.prepare(request)?.executable != *expected {
            return Err(Error::Invalid(super::constants::CHANGED_EXECUTABLE.into()));
        }
    }
    let sources: Vec<_> = request
        .files
        .iter()
        .map(|file| request.root.join(file.0.as_ref()))
        .collect();
    let before = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&sources))?;
    let mut pair = measure(
        &certificate.coverage,
        reference,
        candidate,
        execution,
        request,
        order,
    )?;
    if execution.profile() != certificate.execution {
        return Err(Error::Invalid(
            super::constants::CHANGED_INSTRUMENTATION.into(),
        ));
    }
    if suite.fingerprint(crate::outbound::suite::types::ArtifactSet(
        &certificate.paths,
    ))? != certificate.fingerprint
    {
        return Err(Error::Invalid(super::constants::CHANGED_DURING_RUN.into()));
    }
    if suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&sources))? != before {
        return Err(Error::Invalid(super::constants::CHANGED_SOURCE.into()));
    }
    pair.eligibility = match mode {
        crate::domain::measurements::types::MeasurementMode::Benchmark { .. } => {
            crate::domain::measurements::types::MeasurementEligibility::CompleteInventory
        }
        crate::domain::measurements::types::MeasurementMode::Diagnostic => {
            crate::domain::measurements::types::MeasurementEligibility::Diagnostic
        }
    };
    Ok(pair)
}
