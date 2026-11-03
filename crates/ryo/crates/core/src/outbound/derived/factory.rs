//! Composition edge for the portable implementation.
use super::interface::DerivedPort;
#[must_use]
pub fn use_case(
    semantics: impl ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase
    + Send
    + Sync
    + std::fmt::Debug,
) -> impl DerivedPort {
    super::adapters::use_case::adapter::DerivedUseCaseAdapter::new(semantics)
}
#[cfg(test)]
pub(crate) fn test_port() -> std::sync::Arc<dyn DerivedPort> {
    std::sync::Arc::new(use_case(
        ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
    ))
}
