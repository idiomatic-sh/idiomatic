//! The public admission over the real audit operation.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::admit_measurement::{
    fixtures::tool, interface::AdmitMeasurementUseCase as _, types::MeasurementAdmission,
    use_case::AdmitMeasurementUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::audit::fixtures::audited;
use ryo_benchmark_contracts::domain::measurements::types::MeasurementMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;

#[then("admit_measurement admits a diagnostic subset")]
fn diagnostic(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(matches!(
        admit(BenchmarkCoverage::Interim, true),
        Ok(MeasurementMode::Diagnostic)
    ));
}

#[then("admit_measurement refuses an accounted run that authored nothing")]
fn refuses(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(admit(BenchmarkCoverage::Accounted, false).is_err());
}

/// Admit a run of `scope` over an audit that authored the default rule, or authored nothing.
fn admit(scope: BenchmarkCoverage, authored: bool) -> Result<MeasurementMode, ReportsError> {
    let tool = tool();
    let audit = audited(authored);
    AdmitMeasurementUseCaseImpl::new().admit_measurement(&MeasurementAdmission {
        scope,
        audit: &audit,
        tool: &tool,
        declared_inventory: tool.defaults.clone(),
    })
}
