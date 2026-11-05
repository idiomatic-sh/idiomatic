//! Compose the portable operator adapter at an SDK edge.
use ryo_execution::inbound::use_cases::execution::types::OperatorsPort;

/// The stateless use case adapter; the edge constructs it once per lint service.
#[must_use]
pub fn use_case(
    operators: impl ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase
    + Send
    + Sync
    + std::fmt::Debug,
) -> impl OperatorsPort {
    operators
}
