//! Stable scalar observations written through the executable's supplied writer.
use super::commands::constants as runner_constants;
use ryo_benchmark::domain::measurements::types::sample::value::BenchmarkSample;
use ryo_benchmark_contracts::domain::measurements::types::RunOrder;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Format per-rule multiplicities explicitly, without debug representations of nominal types.
pub(super) fn diagnostic_counts(
    report: &ryo_benchmark_contracts::domain::reports::types::RunReport,
) -> FindingCounts<'_> {
    let mut counts = std::collections::BTreeMap::new();
    for diagnostic in &report.diagnostics {
        counts
            .entry(&diagnostic.rule)
            .or_insert_with(FindingCount::default)
            .0 += runner_constants::ONE_FINDING;
    }
    FindingCounts(counts)
}

/// Write scalar child measurements and the explicit execution order.
pub(super) fn observe_sample(
    output: &mut dyn std::io::Write,
    sample: &BenchmarkSample,
    order: RunOrder,
) -> Result<(), Error> {
    use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;
    let order = match order {
        RunOrder::ReferenceFirst => runner_constants::REFERENCE_FIRST,
        RunOrder::CandidateFirst => runner_constants::CANDIDATE_FIRST,
    };
    match sample.measurement().resources {
        ResourceObservation::Collected(resources) => writeln!(
            output,
            "sample: name={} order={order} wall_seconds={} cpu_seconds={} rss_bytes={}",
            sample.name().0,
            sample.measurement().wall.as_secs_f64(),
            resources.cpu.as_secs_f64(),
            resources.peak_rss_bytes.0
        )?,
        ResourceObservation::WallOnly => writeln!(
            output,
            "sample: name={} order={order} wall_seconds={} resources=absent",
            sample.name().0,
            sample.measurement().wall.as_secs_f64()
        )?,
    }
    Ok(())
}

/// Number of finding occurrences for one rule, including duplicates.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FindingCount(pub usize);

/// Record collection semantics and limitations beside, rather than inside, timing output.
pub(super) fn environment(output: &mut dyn std::io::Write) -> Result<(), Error> {
    writeln!(
        output,
        "environment: os={} arch={} release=true workers=1; isolated collector Linux wait4 CPU=user+system, RSS=child peak bytes; child wall includes spawn/wait and pipe collection; collector launch, normalization and certificate/source hashing excluded; OS page cache uncontrolled; retained cache hits unverified",
        std::env::consts::OS,
        std::env::consts::ARCH
    )?;
    Ok(())
}

/// Structured finding multiplicities with an explicit observation representation.
#[derive(Debug)]
pub(super) struct FindingCounts<'a>(
    std::collections::BTreeMap<&'a ryo::linter::RuleId, FindingCount>,
);
impl std::fmt::Display for FindingCounts<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, (rule, count)) in self.0.iter().enumerate() {
            if index != 0_usize {
                f.write_str(runner_constants::FINDING_SEPARATOR)?;
            }
            write!(f, "{rule}={}", count.0)?;
        }
        Ok(())
    }
}
