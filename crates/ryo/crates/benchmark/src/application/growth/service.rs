//! Grow each shape: write its scene, lint the half and the whole program, and refuse a doubling over the limits.
use crate::domain::growth::types::{GrowthPlan, ShapeGrowths};
use crate::outbound::engine::interface::EnginePort;
use crate::outbound::process_meter::interface::ProcessMeterPort;
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::ArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// Lint every shape of `plan` at both sizes through `engine`, fastest of five. Each program's scene is a fresh directory
/// under `scenes` (`SuitePort::retained_directory`), its modules copied in (`copy_file`) and its program written as the
/// plan's one source file (`write_text`); `meter` takes the CPU time around each lint and resets and reads the peak.
///
/// # Errors
/// A scene or a lint that fails, or a shape over the CPU limit (2.2) or the plan's memory bound.
pub fn grow(
    engine: &dyn EnginePort,
    suite: &dyn SuitePort,
    meter: &dyn ProcessMeterPort,
    scenes: ArtifactPath<'_>,
    plan: &GrowthPlan,
) -> Result<ShapeGrowths, Error> {
    let _unused = (engine, suite, meter, scenes, plan);
    Err(Error::Invalid(String::new()))
}
#[cfg(test)]
#[path = "service_test.rs"]
mod tests;
