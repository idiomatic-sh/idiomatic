//! An accounted run shows its inventory; every run shows its authored rules.
use super::types::MeasurementAdmission;
use ryo_benchmark_contracts::domain::measurements::types::MeasurementMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;

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
    let mode = if *scope == BenchmarkCoverage::Accounted {
        MeasurementMode::Benchmark {
            inventory: declared_inventory.clone(),
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
