//! Preparation and decoding are deliberately outside the measured interval.
use super::constants;
use crate::domain::diagnostics::service::validate_coverage;
use crate::domain::rule_context::service::admit_configuration;
use crate::outbound::context::interface::ContextPort;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::interface::LinterPort;
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    ContextSeal, FileMetadataDeclaration, RunRequest,
};

/// Prepare a measurement request with no declared file observations, then validate decoded coverage.
/// Observation-bearing verification uses the `case_execution` preparation boundary.
///
/// # Errors
/// Propagates preparation, launch or decoding errors and refuses invalid input coverage or ranges.
pub fn run(
    context: &dyn ContextPort,
    linter: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
) -> Result<Measurement, Error> {
    run_with(
        context,
        linter,
        execution,
        request,
        super::types::Pinning::Timed,
    )
}

/// Run as `run` does, keeping the thread pin the linter sets for comparable timing only for a timed
/// run; an untimed run drops that one variable, so the child runs at its default threads.
///
/// # Errors
/// As `run`.
pub(crate) fn run_with(
    context: &dyn ContextPort,
    linter: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
    pinning: super::types::Pinning,
) -> Result<Measurement, Error> {
    if request.file_metadata != FileMetadataDeclaration::Undeclared {
        return Err(Error::Invalid(
            constants::DECLARED_FILE_OBSERVATIONS_REQUIRE_VERIFICAT.into(),
        ));
    }
    if let ContextSeal::Sealed(manifest) = &request.context_manifest {
        context.validate(manifest, &request.root)?;
    }
    admit_configuration(request, linter.rule_configuration())?;
    let mut invocation = linter.prepare(request)?;
    if pinning == super::types::Pinning::Untimed {
        let _pin = invocation.environment.remove(std::ffi::OsStr::new(
            ryo_benchmark_contracts::domain::measurements::constants::THREAD_PIN,
        ));
    }
    let output = execution.run(invocation)?;
    let wall = output.wall;
    let resources = output.resources;
    let report = linter.decode(request, output)?;
    if let (
        ContextSeal::Sealed(manifest),
        ryo_benchmark_contracts::domain::reports::types::ProjectContext::Observed(receipt),
    ) = (&request.context_manifest, &report.project_context)
    {
        context.verify_receipt(manifest, &request.root, receipt)?;
    }
    validate_coverage(&request.files, &report)?;
    Ok(Measurement {
        wall,
        resources,
        report,
    })
}
