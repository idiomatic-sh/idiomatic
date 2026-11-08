//! The derived port's fakes, behind the store's proofs and the engine's.

/// The derived capability behind its port, with the call counter the port's fake keeps.
#[must_use]
pub fn derived() -> crate::outbound::derived::fixtures::fake::DerivedFake {
    crate::outbound::derived::fixtures::fake::DerivedFake::default()
}

/// The derived capability as the engine holds it.
#[must_use]
pub fn program_derived_port() -> std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort> {
    std::sync::Arc::new(derived())
}
