//! Stable scalar observations written through the executable's supplied writer.
use super::constants;
use ryo_benchmark::domain::diagnostics::errors::DiagnosticsError as Error;
use ryo_benchmark::domain::measurements::types::{RunOrder, sample::value::BenchmarkSample};
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
    match sample.measurement().resources {
        ResourceObservation::Collected(resources) => writeln!(
            output,
            "sample: name={} order={order:?} wall_seconds={} cpu_seconds={} rss_bytes={}",
            sample.name().0,
            sample.measurement().wall.as_secs_f64(),
            resources.cpu.as_secs_f64(),
            resources.peak_rss_bytes.0
        )?,
        ResourceObservation::WallOnly => writeln!(
            output,
            "sample: name={} order={order:?} wall_seconds={} resources=absent",
            sample.name().0,
            sample.measurement().wall.as_secs_f64()
        )?,
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FindingCount(pub usize);

pub(super) fn environment(output: &mut dyn std::io::Write) -> Result<(), Error> {
    writeln!(
        output,
        "environment: os={} arch={} release=true workers=1; isolated collector Linux wait4 CPU=user+system, RSS=child peak bytes; child wall includes spawn/wait and pipe collection; collector launch, normalization and certificate/source hashing excluded; OS page cache uncontrolled; retained cache hits unverified",
        std::env::consts::OS,
        std::env::consts::ARCH
    )?;
    Ok(())
}
