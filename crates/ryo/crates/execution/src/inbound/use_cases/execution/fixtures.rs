//! The test support of the `execution` use case: the scenario its tests share.
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_execution_store::outbound::derived::interface::DerivedPort;
use ryo_matching::inbound::use_cases::matching::{
    errors::MatchingError, interface::MatchingUseCase, types::RegexSource,
    use_case::MatchingUseCaseImpl,
};
use std::sync::Arc;

#[cfg(test)]
pub(crate) mod requests;

/// The real operators capability, as the engine composes it.
#[must_use]
pub fn operators() -> impl OperatorsPort {
    ryo_execution_runtime::outbound::operators::factory::use_case(
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
            Arc::new(ryo_operators::outbound::matching::factory::use_case()),
        ),
    )
}
/// The refusal the engine gives a regular expression the matcher cannot compile, in the
/// matcher's words; `None` when `pattern` compiles.
#[must_use]
pub fn regex_refusal(pattern: &str) -> Option<OperatingError> {
    let error = MatchingUseCaseImpl::new()
        .compile_regex(RegexSource(pattern))
        .err()?;
    let (MatchingError::InvalidRegex { reason } | MatchingError::BadGlob { reason, .. }) = error;
    Some(OperatingError::InvalidRegex { reason })
}
/// The derived capability behind its port, as the edge composes it.
#[must_use]
pub fn derived() -> impl DerivedPort {
    ryo_execution_store::outbound::derived::factory::semantics()
}
