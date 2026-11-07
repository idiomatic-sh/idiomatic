//! Builds the manifest adapters, one function each. A factory builds nothing else, chooses nothing, and is called only
//! by an edge.

/// The TOML adapter: `asymptote.toml` read from the disk.
#[must_use]
pub fn toml() -> impl crate::outbound::manifest::interface::ManifestPort {
    super::adapters::toml::adapter::ManifestTomlAdapter
}
