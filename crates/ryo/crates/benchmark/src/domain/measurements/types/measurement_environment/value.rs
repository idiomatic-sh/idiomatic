//! The collection semantics and limitations a measurement states beside, rather than inside, its timing output.

/// The measurement environment of this machine, stated as one observation line.
#[derive(Clone, Copy, Debug, Default)]
pub struct MeasurementEnvironment;

impl std::fmt::Display for MeasurementEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "environment: os={} arch={} release=true workers=1; isolated collector Linux wait4 CPU=user+system, RSS=child peak bytes; child wall includes spawn/wait and pipe collection; collector launch, normalization and certificate/source hashing excluded; OS page cache uncontrolled; retained cache hits unverified",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    }
}
