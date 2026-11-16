//! Constructs the one SDK engine adapter with explicit diagnostic projection and tracing.
use super::adapters::local::adapter::EngineLocalAdapter;
use super::types::FrontendDiagnosticProjection;

/// Construct the SDK engine. Tracing is a separate diagnostic worker configuration.
#[must_use]
pub fn local(
    languages: ryo::linter::Languages,
    trace: Option<std::path::PathBuf>,
) -> impl super::interface::EnginePort {
    EngineLocalAdapter::new(languages, trace, None)
}

/// Construct the SDK engine with an explicit query for selected frontend diagnostics.
#[must_use]
pub fn local_with_frontend_diagnostic(
    languages: ryo::linter::Languages,
    trace: Option<std::path::PathBuf>,
    projection: FrontendDiagnosticProjection,
) -> impl super::interface::EnginePort {
    EngineLocalAdapter::new(languages, trace, Some(projection))
}

/// Construct the engine with a synchronous JSONL event sink for bounded diagnostic runs.
#[must_use]
pub fn local_with_diagnostic_events(
    languages: ryo::linter::Languages,
    events: std::path::PathBuf,
    projection: FrontendDiagnosticProjection,
) -> impl super::interface::EnginePort {
    EngineLocalAdapter::with_diagnostic_events(languages, events, Some(projection))
}

/// Construct an engine for a verification: the rules load once and every case runs over one SDK engine, so the plans compile
/// once and the cases share them.
#[must_use]
pub fn retaining_with_frontend_diagnostic(
    languages: ryo::linter::Languages,
    projection: FrontendDiagnosticProjection,
) -> impl super::interface::EnginePort {
    EngineLocalAdapter::retaining(languages, Some(projection))
}
