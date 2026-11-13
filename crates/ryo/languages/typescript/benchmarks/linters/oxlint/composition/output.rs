//! Stable scalar observations written through the executable's supplied writer.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

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
