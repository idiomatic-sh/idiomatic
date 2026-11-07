//! Maps the operators' matching port to the matching crate's use case.

use crate::outbound::matching::interface::MatchingPort;
use ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
use ryo_matching::inbound::use_cases::matching::types::{GroupName, SearchText, TextSpan};

/// Adapts the matching use case to the port the operators call.
#[derive(Debug)]
pub struct MatchingUseCaseAdapter<Semantics> {
    semantics: Semantics,
}

impl<Semantics> MatchingUseCaseAdapter<Semantics> {
    /// The adapter over one use case.
    pub const fn new(semantics: Semantics) -> Self {
        Self { semantics }
    }
}

impl<Semantics> MatchingPort for MatchingUseCaseAdapter<Semantics>
where
    Semantics: MatchingUseCase + Send + Sync,
{
    fn visit_regex_matches(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        each: &mut dyn FnMut(TextSpan) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()> {
        self.semantics.visit_regex_matches(regex, text, each)
    }

    fn visit_regex_captures(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        name: GroupName<'_>,
        each: &mut dyn FnMut(TextSpan, TextSpan) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, MatchingError> {
        self.semantics.visit_regex_captures(regex, text, name, each)
    }
}
