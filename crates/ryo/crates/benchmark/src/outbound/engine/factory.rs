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

/// Construct the SDK engine wiring only the execution strategies `routes` names, for a differential run.
#[cfg(feature = "fixtures")]
#[must_use]
pub fn local_with_routes(
    languages: ryo::linter::Languages,
    projection: FrontendDiagnosticProjection,
    routes: ryo::linter::RoutePolicy,
) -> impl super::interface::EnginePort {
    EngineLocalAdapter::new(languages, None, Some(projection)).routing(routes)
}
