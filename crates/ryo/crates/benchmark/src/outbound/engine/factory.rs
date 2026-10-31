//! Constructs the one SDK engine adapter with explicit optional diagnostic tracing.
use super::adapters::local::adapter::EngineLocalAdapter;
/// Construct the SDK engine. Tracing is a separate diagnostic worker configuration.
#[must_use]
pub fn local(
    languages: ryo::linter::Languages,
    trace: Option<std::path::PathBuf>,
) -> impl super::interface::EnginePort {
    EngineLocalAdapter::new(languages, trace)
}
