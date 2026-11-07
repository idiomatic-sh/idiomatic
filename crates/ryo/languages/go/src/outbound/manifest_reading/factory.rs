//! Builds the manifest reading adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `gomod` manifest reading adapter: the manifests of Go, `go.mod` and `go.work`, read by the line grammar `cmd/go` reads them with.
#[must_use]
pub(crate) fn gomod() -> impl crate::outbound::manifest_reading::interface::ManifestReadingPort {
    super::adapters::gomod::adapter::ManifestReadingGomodAdapter
}
