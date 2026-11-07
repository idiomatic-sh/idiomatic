//! What the TOML lock adapter is configured with.

use crate::domain::manifest::types::ProductRoot;

/// The TOML lock adapter's settings: the values and parts its
/// composition chooses.
#[derive(Debug)]
pub(crate) struct LockTomlSettings {
    /// The product whose lock it reads and writes.
    pub root: ProductRoot,
}
