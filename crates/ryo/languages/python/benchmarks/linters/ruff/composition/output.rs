//! Stable scalar observations written through the executable's supplied writer.
use super::constants;
use ryo_benchmark::domain::diagnostics::errors::DiagnosticsError as Error;
use ryo_benchmark::domain::measurements::types::{BenchmarkSample, RunOrder};
pub(super) fn diagnostic_counts(
    report: &ryo_benchmark::domain::diagnostics::types::RunReport,
) -> std::collections::BTreeMap<&ryo::linter::RuleId, FindingCount> {
    let mut counts = std::collections::BTreeMap::new();
    for diagnostic in &report.diagnostics {
        counts
            .entry(&diagnostic.rule)
            .or_insert_with(FindingCount::default)
            .0 += constants::ONE_FINDING;
    }
    counts
}

pub(super) fn observe_sample(
    output: &mut dyn std::io::Write,
    sample: &BenchmarkSample,
    order: RunOrder,
) -> Result<(), Error> {
    use ryo_benchmark::domain::measurements::types::ResourceObservation;
    match sample.measurement.resources {
        ResourceObservation::Collected(resources) => writeln!(
            output,
            "sample: name={} order={order:?} wall_seconds={} cpu_seconds={} rss_bytes={}",
            sample.name.0,
            sample.measurement.wall.as_secs_f64(),
            resources.cpu.as_secs_f64(),
            resources.peak_rss_bytes.0
        )?,
        ResourceObservation::WallOnly => writeln!(
            output,
            "sample: name={} order={order:?} wall_seconds={} resources=absent",
            sample.name.0,
            sample.measurement.wall.as_secs_f64()
        )?,
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FindingCount(pub usize);
