//! Constructs the one SDK engine adapter with explicit optional diagnostic tracing.
use super::adapters::local::adapter::EngineLocalAdapter;
/// Construct the SDK engine. Tracing is a separate diagnostic worker configuration.
#[must_use]
pub fn local(
    languages: ryo::linter::Languages,
    trace: Option<std::path::PathBuf>,
) -> impl super::interface::EnginePort {
    EngineLocalAdapter::new(languages, trace, None)
}

/// Construct the SDK engine with an optional normalized frontend diagnostic identity.
#[must_use]
pub fn local_with_frontend_diagnostic(
    languages: ryo::linter::Languages,
    trace: Option<std::path::PathBuf>,
    identity: ryo::linter::RuleId,
) -> impl super::interface::EnginePort {
    EngineLocalAdapter::new(languages, trace, Some(identity))
}
