//! Builds the manifest adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `toml` manifest adapter.
#[must_use]
pub(crate) fn toml() -> impl crate::outbound::manifest::interface::ManifestPort {
    super::adapters::toml::adapter::ManifestTomlAdapter
}
