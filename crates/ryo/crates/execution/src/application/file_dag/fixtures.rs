//! The port fakes of the operators and derived capabilities, behind the file DAG's proofs.

/// The operators capability behind its port, with the call counter the port's fake keeps.
pub(crate) fn operators() -> crate::outbound::operators::fixtures::fake::OperatorsFake {
    crate::outbound::operators::fixtures::fake::OperatorsFake::default()
}

/// The derived capability behind its port, with the call counter the port's fake keeps.
pub(crate) fn derived() -> crate::outbound::derived::fixtures::fake::DerivedFake {
    crate::outbound::derived::fixtures::fake::DerivedFake::default()
}

/// The derived capability as the engine holds it.
pub(crate) fn program_derived_port()
-> std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort> {
    std::sync::Arc::new(derived())
}

/// The derived capability with its real portable semantics, behind its port.
pub(crate) fn real_derived_port() -> std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort> {
    std::sync::Arc::new(crate::outbound::derived::factory::semantics())
}
