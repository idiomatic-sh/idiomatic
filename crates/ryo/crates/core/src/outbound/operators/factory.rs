//! Compose the portable operator adapter at an SDK edge.
use super::interface::OperatorsPort;

/// The stateless use case adapter; the edge constructs it once per lint service.
#[must_use]
pub fn use_case(
    operators: impl ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase
    + Send
    + Sync
    + std::fmt::Debug,
) -> impl OperatorsPort {
    super::adapters::use_case::adapter::OperatorsUseCaseAdapter::new(operators)
}

/// Test-only composition of the same stateless capability boundary.
#[cfg(test)]
pub(crate) const fn test_port() -> &'static dyn OperatorsPort {
    use super::adapters::use_case::adapter::OperatorsUseCaseAdapter;
    use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;

    static ADAPTER: OperatorsUseCaseAdapter<OperatorsUseCaseImpl> =
        OperatorsUseCaseAdapter::new(OperatorsUseCaseImpl::new());
    &ADAPTER
}
