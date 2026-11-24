//! An accounted run shows its inventory; every run shows its authored rules.
use super::types::MeasurementAdmission;
use benchmark_contracts::domain::measurements::types::MeasurementMode;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::requests::types::BenchmarkCoverage;

/// The mode the run is measured in, after the accounted run has shown its inventory and every run its authored rules.
pub(super) fn admit(admission: &MeasurementAdmission<'_>) -> Result<MeasurementMode, Error> {
    let MeasurementAdmission {
        scope,
        audit,
        tool,
        declared_inventory,
    } = admission;
    if *scope == BenchmarkCoverage::Accounted {
        crate::domain::suite::service::require_accounted(audit, tool)?;
    }
    // The accounted run measures the declared inventory but the defaults the profile defers: a deferred default has no
    // authored query, so it is never selected, and the accounting above has already held each deferral to its capability.
    let mode = if *scope == BenchmarkCoverage::Accounted {
        MeasurementMode::Benchmark {
            inventory: declared_inventory
                .iter()
                .filter(|rule| !tool.deferred.contains_key(*rule))
                .cloned()
                .collect(),
        }
    } else {
        MeasurementMode::Diagnostic
    };
    crate::domain::measurements::service::require_inventory(
        &mode,
        audit.authored(),
        Some(audit.authored()),
    )?;
    Ok(mode)
}
