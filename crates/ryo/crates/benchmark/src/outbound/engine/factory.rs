//! Builds the engine adapters from dependencies chosen by the composition.
use super::adapters::local::adapter::EngineLocalAdapter;
/// Construct the local implementation of the engine contract.
#[must_use]
pub fn local(languages: ryo::linter::Languages) -> impl super::interface::EnginePort {
    EngineLocalAdapter::new(languages)
}

/// Record existing engine phase spans for a diagnostic run, separate from performance samples.
#[must_use]
pub fn profiled(
    languages: ryo::linter::Languages,
    trace: std::path::PathBuf,
) -> impl super::interface::EnginePort {
    EngineLocalAdapter::profiled(languages, trace)
}
