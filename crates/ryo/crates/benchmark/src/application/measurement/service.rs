//! Preparation and decoding are deliberately outside the measured interval.
use super::constants;
use super::types::FindingCounts;
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
    if request.file_metadata != FileMetadataDeclaration::Undeclared {
        return Err(Error::Invalid(
            constants::DECLARED_FILE_OBSERVATIONS_REQUIRE_VERIFICAT.into(),
        ));
    }
    if let ContextSeal::Sealed(manifest) = &request.context_manifest {
        context.validate(manifest, &request.root)?;
    }
    admit_configuration(request, linter.rule_configuration())?;
    let invocation = linter.prepare(request)?;
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
    let load = crate::outbound::profile_tools::standard::load()
        .map_or_else(|| "unavailable".to_owned(), |load| format!("{load:.2}"));
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
    Ok(())
}
