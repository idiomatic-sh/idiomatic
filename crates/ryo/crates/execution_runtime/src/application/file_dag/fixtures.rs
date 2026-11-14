//! The port fakes of the operators and derived capabilities, behind the file DAG's proofs.

/// The operators capability behind its port, with the call counter the port's fake keeps.
pub fn operators() -> crate::outbound::operators::fixtures::fake::OperatorsFake {
    crate::outbound::operators::fixtures::fake::OperatorsFake::default()
}

/// The derived capability with its real portable semantics, behind its port.
pub fn real_derived_port()
-> std::sync::Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort> {
    std::sync::Arc::new(ryo_execution_store::outbound::derived::factory::semantics())
}

/// The matching semantics the proofs hand to the domain: the portable matching use case.
pub fn pattern_matcher() -> ryo_operators::domain::patterns::types::PatternMatcher<'static> {
    ryo_operators::domain::patterns::types::PatternMatcher(
        &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
    )
}
#[cfg(any(test, feature = "fixtures"))]
pub mod operator_contract;
