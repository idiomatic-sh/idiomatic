//! The port fakes of the operators capability and the matching semantics, behind the file DAG's proofs.

/// The operators capability behind its port, with the call counter the port's fake keeps.
pub fn operators() -> crate::outbound::operators::fixtures::fake::OperatorsFake {
    crate::outbound::operators::fixtures::fake::OperatorsFake::default()
}

/// The matching semantics the proofs hand to the domain: the portable matching use case.
pub fn pattern_matcher() -> ryo_operators::domain::patterns::types::PatternMatcher<'static> {
    ryo_operators::domain::patterns::types::PatternMatcher(
        &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
    )
}
#[cfg(any(test, feature = "fixtures"))]
pub mod operator_contract;
