//! A measurement is certified only while its instrumentation, artifacts and sources stay as certified.
use super::{constants, types::RunBaseline};
use crate::domain::diagnostics::service::compare_with_contract;
use crate::domain::measurements::types::{MeasurementEligibility, pair::value::Pair};
use crate::domain::suite::types::certificate::value::Certificate;
use crate::outbound::evidence::interface::EvidencePort;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::interface::LinterPort;
use crate::outbound::suite::{interface::SuitePort, types::ArtifactSet};
use ryo_benchmark_contracts::domain::measurements::types::{Measurement, MeasurementMode};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::requests::types::{CohortSelection, RunRequest};

/// Require the certified inventory, instrumentation, artifacts, commands and executables before a run.
///
/// # Errors
/// Refuses any certified identity that differs from the one the contestants and the suite hold now.
pub fn admit(
    certificate: &Certificate,
    mode: &MeasurementMode,
    suite: &dyn SuitePort,
    execution: &dyn ExecutionPort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    request: &RunRequest,
) -> Result<RunBaseline, Error> {
    crate::domain::measurements::service::require_inventory(
        mode,
        certificate.coverage.authored(),
        match &request.selected_rules {
            CohortSelection::Explicit(rules) => Some(rules.as_slice()),
            CohortSelection::Frozen => None,
        },
    )?;
    if execution.profile() != certificate.execution {
        return Err(Error::Invalid(constants::CHANGED_INSTRUMENTATION.into()));
    }
    if suite.fingerprint(ArtifactSet(&certificate.paths))? != certificate.fingerprint {
        return Err(Error::Invalid(constants::CHANGED_EVIDENCE.into()));
    }
    if [
        reference.prepare(&certificate.probe)?,
        candidate.prepare(&certificate.probe)?,
    ] != certificate.profiles
    {
        return Err(Error::Invalid(constants::CHANGED_COMMAND.into()));
    }
    for (tool, expected) in [
        (reference, &certificate.contestants[0]),
        (candidate, &certificate.contestants[1]),
    ] {
        if tool.prepare(request)?.executable != *expected {
            return Err(Error::Invalid(constants::CHANGED_EXECUTABLE.into()));
        }
    }
    let sources: Vec<_> = request
        .files
        .iter()
        .map(|file| request.root.join(file.0.as_ref()))
        .collect();
    let before = suite.fingerprint(ArtifactSet(&sources))?;
    Ok(RunBaseline { sources, before })
}

/// Require both contestants' reports to agree on every finding and publish the comparison evidence.
///
/// # Errors
/// Refuses reports that differ under the diagnostics contract.
pub fn pair(
    evidence: &dyn EvidencePort,
    reference: Measurement,
    candidate: Measurement,
) -> Result<Pair, Error> {
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
    evidence.comparison(
        &reference.report,
        &candidate.report,
        comparison,
        ComparisonContract::Diagnostics,
    )?;
    Ok(Pair {
        eligibility: MeasurementEligibility::Diagnostic,
        reference,
        candidate,
    })
}

/// Require the instrumentation, certified artifacts and sources unchanged and issue the pair's eligibility.
///
/// # Errors
/// Refuses any identity that changed while the contestants ran.
pub fn conclude(
    certificate: &Certificate,
    mode: &MeasurementMode,
    suite: &dyn SuitePort,
    execution: &dyn ExecutionPort,
    baseline: &RunBaseline,
    mut pair: Pair,
) -> Result<Pair, Error> {
    if execution.profile() != certificate.execution {
        return Err(Error::Invalid(constants::CHANGED_INSTRUMENTATION.into()));
    }
    if suite.fingerprint(ArtifactSet(&certificate.paths))? != certificate.fingerprint {
        return Err(Error::Invalid(constants::CHANGED_DURING_RUN.into()));
    }
    if suite.fingerprint(ArtifactSet(&baseline.sources))? != baseline.before {
        return Err(Error::Invalid(constants::CHANGED_SOURCE.into()));
    }
    pair.eligibility = match mode {
        MeasurementMode::Benchmark { .. } => MeasurementEligibility::CompleteInventory,
        MeasurementMode::Diagnostic => MeasurementEligibility::Diagnostic,
    };
    Ok(pair)
}

/// Require the certified coverage to be complete before any contestant runs.
///
/// # Errors
/// Refuses a certificate whose audit leaves a default rule missing.
pub fn require_covered(certificate: &Certificate) -> Result<(), Error> {
    crate::domain::suite::service::require_complete(&certificate.coverage)
}
