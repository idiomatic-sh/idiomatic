//! What the postcard check store adapter is configured with.

use crate::outbound::check_store::types::Build;

/// The postcard check store adapter's settings: the values and parts its
/// composition chooses.
#[derive(Debug)]
pub(crate) struct CheckStorePostcardSettings {
    /// The store's file.
    pub file: std::path::PathBuf,
    /// The build that runs: a store another build wrote reads as empty.
    pub build: Build,
}
