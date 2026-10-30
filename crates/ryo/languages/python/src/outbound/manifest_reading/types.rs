//! The types of the `manifest_reading` port.

/// Every manifest of a run's repository: what the files a requirements file includes are read from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AvailableManifests(pub Vec<ryo::languages::ManifestFile>);

/// Whether an adapter reads a file as one of its manifests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Claim {
    /// It does.
    Reads,
    /// It does not.
    Skips,
}
