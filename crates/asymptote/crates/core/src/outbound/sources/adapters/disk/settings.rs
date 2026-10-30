//! What the disk sources adapter is configured with.

use crate::domain::manifest::types::ProductRoot;

/// The disk sources adapter's settings: the values and parts its
/// composition chooses.
#[derive(Debug)]
pub(crate) struct SourcesDiskSettings {
    /// The product every path is relative to.
    pub root: ProductRoot,
}
