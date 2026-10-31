//! Builds the engine adapters from dependencies chosen by the composition.
use super::adapters::local::adapter::EngineLocalAdapter;
/// Construct the local implementation of the engine contract.
#[must_use]
pub fn local(languages: ryo::linter::Languages) -> impl super::interface::EnginePort {
    EngineLocalAdapter { languages }
}
