//! What one rule's verification keeps on disk.
use crate::outbound::evidence::types::VerificationTimesFile;
use std::path::Path;

/// Where one rule's verification keeps its observations, and the file its verification times go to.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RuleFiles<'a> {
    /// The rule's observation directory.
    pub directory: &'a Path,
    /// The rule's verification times file.
    pub times: &'a VerificationTimesFile,
}
