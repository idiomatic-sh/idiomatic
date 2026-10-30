//! Builds the conditions adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `cargo-platform` conditions adapter.
#[must_use]
pub(crate) fn cargo_platform() -> impl crate::outbound::conditions::interface::ConditionsPort {
    super::adapters::cargo_platform::adapter::ConditionsCargoPlatformAdapter
}
