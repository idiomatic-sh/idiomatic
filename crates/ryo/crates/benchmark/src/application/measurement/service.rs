//! Preparation and decoding are deliberately outside the measured interval.
use super::types::FindingCounts;
use super::{constants, phases};
use crate::domain::diagnostics::service::validate_coverage;
use crate::domain::rule_context::service::admit_configuration;
use crate::outbound::context::interface::ContextPort;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::interface::LinterPort;
use ryo_benchmark_contracts::domain::costs::types::RunCost;
use ryo_benchmark_contracts::domain::measurements::types::{
    Measurement, ResourceObservation, RunOrder,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::constants::RUN_RYO;
use ryo_benchmark_contracts::domain::requests::types::{
    ContextSeal, DiagnosticSwitch, FileMetadataDeclaration, RunRequest,
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
    if request.file_metadata != FileMetadataDeclaration::Undeclared {
        return Err(Error::Invalid(
            constants::DECLARED_FILE_OBSERVATIONS_REQUIRE_VERIFICAT.into(),
        ));
    }
    if let ContextSeal::Sealed(manifest) = &request.context_manifest {
        context.validate(manifest, &request.root)?;
    }
    admit_configuration(request, linter.rule_configuration())?;
    let mut measured = request.clone();
    measured.performance.diagnostics.execution = DiagnosticSwitch::On;
    let invocation = linter.prepare(&measured)?;
    let phase_path = invocation
        .arguments
        .first()
        .filter(|argument| *argument == RUN_RYO)
        .and_then(|_| phases::observation_file(&request.state));
    if let Some(path) = &phase_path
        && let Err(error) = std::fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(error.into());
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
        phases: phase_path
            .as_deref()
            .map(phases::read)
            .transpose()?
            .flatten(),
        report,
    })
}

/// The candidate's CPU, wall and peak of one condition across the trials.
///
/// # Errors
/// Refuses a run whose resources were not collected.
pub(crate) fn cost(runs: &[Measurement]) -> Result<RunCost, Error> {
    let mut cost = RunCost {
        cpu: Vec::new(),
        wall: Vec::new(),
        peak: Vec::new(),
    };
    for run in runs {
        let ResourceObservation::Collected(resources) = run.resources else {
            return Err(Error::Invalid(constants::NO_RESOURCES.into()));
        };
        cost.cpu.push(resources.cpu);
        cost.wall.push(run.wall);
        cost.peak.push(resources.peak_rss_bytes);
    }
    Ok(cost)
}

/// Count the findings of each rule, duplicates included.
pub(crate) fn finding_counts(report: &RunReport) -> FindingCounts<'_> {
    let mut counts = std::collections::BTreeMap::new();
    for diagnostic in &report.diagnostics {
        *counts.entry(&diagnostic.rule).or_insert(0_usize) += constants::ONE_FINDING;
    }
    FindingCounts(counts)
}

/// Write one run's scalar measurements and the explicit execution order.
///
/// # Errors
/// A write the observations stream refuses.
pub(crate) fn observe(
    output: &mut dyn std::io::Write,
    name: &ryo_benchmark_contracts::domain::measurements::types::SampleName,
    measurement: &Measurement,
    order: RunOrder,
) -> Result<(), Error> {
    let order = match order {
        RunOrder::ReferenceFirst => constants::REFERENCE_FIRST,
        RunOrder::CandidateFirst => constants::CANDIDATE_FIRST,
    };
    let load =
        profile_tools::load().map_or_else(|| "unavailable".to_owned(), |load| format!("{load:.2}"));
    match measurement.resources {
        ResourceObservation::Collected(resources) => writeln!(
            output,
            "sample: name={} order={order} wall_seconds={} cpu_seconds={} rss_bytes={} load_1m={load}",
            name.0,
            measurement.wall.as_secs_f64(),
            resources.cpu.as_secs_f64(),
            resources.peak_rss_bytes.0
        )?,
        ResourceObservation::WallOnly => writeln!(
            output,
            "sample: name={} order={order} wall_seconds={} resources=absent load_1m={load}",
            name.0,
            measurement.wall.as_secs_f64()
        )?,
    }
    if let ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
        coverage,
    ) = measurement.report.coverage
    {
        writeln!(
            output,
            "pairs: name={} order={order} selected={} evaluated={} skipped={} unavailable={}",
            name.0,
            coverage.selected.0,
            coverage.evaluated.0,
            coverage.skipped.0,
            coverage.unknown_or_unread.0,
        )?;
    }
    if let Some(clock) = measurement.phases {
        writeln!(
            output,
            "phases: name={} order={order} preparation_ns={} admission_ns={} sdk_execution_ns={} delivery_ns={} load_1m={load}",
            name.0,
            clock.preparation.as_nanos(),
            clock.admission.as_nanos(),
            clock.execution.as_nanos(),
            clock.delivery.as_nanos(),
        )?;
    }
    Ok(())
}
