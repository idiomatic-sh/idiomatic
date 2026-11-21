//! Builds the engine adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

use crate::outbound::engine::adapters::use_case::adapter::EngineUseCaseAdapter;
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
use ryo_engine::inbound::use_cases::project::use_case::ProjectUseCaseImpl;
use ryo_engine::inbound::use_cases::scan::use_case::ScanUseCaseImpl;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use std::sync::Arc;

/// The adapter over the engine's three use cases, which hold no port: the edge builds it once for a lint service.
///
/// # Examples
///
/// ```
/// let _port = ryo_execution_runtime::outbound::engine::factory::use_case();
/// ```
#[must_use]
pub fn use_case() -> impl EnginePort {
    EngineUseCaseAdapter::new(
        LoadUseCaseImpl::new(),
        ScanUseCaseImpl::new(Arc::new(MatchingUseCaseImpl)),
        ProjectUseCaseImpl::new(),
    )
}
