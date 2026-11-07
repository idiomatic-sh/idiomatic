//! Compose the portable operator adapter at an SDK edge.
use crate::outbound::operators::adapters::use_case::adapter::OperatorsUseCaseAdapter;
use crate::outbound::operators::interface::OperatorsPort;

/// The adapter over the operators use case; the edge constructs it once per lint service.
#[must_use]
pub fn use_case(
    operators: impl ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase
    + Send
    + Sync
    + std::fmt::Debug,
) -> impl OperatorsPort {
    OperatorsUseCaseAdapter::new(operators)
}
