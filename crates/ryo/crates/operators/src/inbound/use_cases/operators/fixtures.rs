//! The test support of the `operators` use case: the capability and the matcher its tests share.
use super::interface::OperatorsUseCase;
use super::use_case::OperatorsUseCaseImpl;
use ryo_matching::inbound::use_cases::matching::{
    compiled::CompiledRegex, interface::MatchingUseCase, types::RegexSource,
    use_case::MatchingUseCaseImpl,
};

/// The operators capability composed as an edge composes it.
#[must_use]
pub fn operators() -> impl OperatorsUseCase {
    OperatorsUseCaseImpl::new(std::sync::Arc::new(crate::outbound::matching::factory::use_case()))
}

/// A compiled regex of an authored pattern the tests know to be valid.
///
/// # Panics
/// Panics when the pattern is not valid regex syntax.
#[must_use]
pub fn regex(pattern: &str) -> CompiledRegex {
    MatchingUseCaseImpl::new()
        .compile_regex(RegexSource(pattern))
        .expect("valid fixture pattern")
}
