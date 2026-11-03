//! Compose the portable operator adapter at an SDK edge.
use super::interface::OperatorsPort;

/// The stateless use case adapter; the edge constructs it once per lint service.
#[must_use]
pub fn use_case() -> impl OperatorsPort {
    super::adapters::use_case::adapter::OperatorsUseCaseAdapter::new()
}
