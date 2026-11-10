//! The admission holds no boundary: its facts are the audit and the tool it is given.
use super::{interface::AdmitMeasurementUseCase, service::admit, types::MeasurementAdmission};
use ryo_benchmark_contracts::domain::measurements::types::MeasurementMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Admits one measurement against its audit.
#[derive(Clone, Copy, Debug, Default)]
pub struct AdmitMeasurementUseCaseImpl;
impl AdmitMeasurementUseCaseImpl {
    /// Construct the admission; it has no boundary to bind.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
impl AdmitMeasurementUseCase for AdmitMeasurementUseCaseImpl {
    fn admit_measurement(
        &self,
        admission: &MeasurementAdmission<'_>,
    ) -> Result<MeasurementMode, Error> {
        admit(admission)
    }
}
