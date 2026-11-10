//! A measurement mode is only ever issued from an audit that holds it.
use super::types::MeasurementAdmission;
use ryo_benchmark_contracts::domain::measurements::types::MeasurementMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
/// Decide the mode one run is measured in.
pub trait AdmitMeasurementUseCase {
    /// An accounted run needs its inventory accounted for and equal to the authored rules; a
    /// diagnostic one is held to nothing.
    ///
    /// # Errors
    /// Refuses an accounted run whose defaults are not all authored, deferred or pending.
    fn admit_measurement(
        &self,
        admission: &MeasurementAdmission<'_>,
    ) -> Result<MeasurementMode, ReportsError>;
}
