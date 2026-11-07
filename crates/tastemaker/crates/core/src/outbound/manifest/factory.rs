//! Builds the manifest adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by
//! an edge.

/// The TOML adapter: `toml` reads, `toml_edit` writes, keeping a file's layout.
#[must_use]
pub fn toml() -> impl crate::outbound::manifest::interface::ManifestPort {
    super::adapters::toml::adapter::ManifestTomlAdapter
}
