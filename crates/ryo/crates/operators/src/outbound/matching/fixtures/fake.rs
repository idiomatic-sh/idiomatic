//! The fake of the `matching` port: what the operators' tests stand in for it with. Matching is pure,
//! so the fake answers with the matching crate's own use case.

use crate::outbound::matching::interface::MatchingPort;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
use ryo_matching::inbound::use_cases::matching::types::CompiledRegex;
use ryo_matching::inbound::use_cases::matching::types::{GroupName, SearchText, TextSpan};
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;

/// What the operators' tests stand in for `matching` with.
#[derive(Clone, Copy, Debug)]
pub struct MatchingFake;

impl MatchingPort for MatchingFake {
    fn visit_regex_matches(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        each: &mut dyn FnMut(TextSpan) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()> {
        MatchingUseCaseImpl::new().visit_regex_matches(regex, text, each)
    }

    fn visit_regex_captures(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        name: GroupName<'_>,
        each: &mut dyn FnMut(TextSpan, TextSpan) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, MatchingError> {
        MatchingUseCaseImpl::new().visit_regex_captures(regex, text, name, each)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
