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

/// Results of one requested manifest read, with independent projection and validation.
pub(crate) struct Reading {
    /// None means projection was not requested. A requested projection is independently successful or refused.
    pub declaration: Option<
        Result<crate::domain::packages::types::Declared, super::errors::ManifestReadingError>,
    >,
    /// None means this adapter does not supply the requested metadata validator.
    pub validation: Option<crate::domain::manifest_validation::types::Validation>,
}
