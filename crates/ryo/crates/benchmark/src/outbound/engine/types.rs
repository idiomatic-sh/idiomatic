//! Prepared inputs for the outbound SDK engine port.
use ryo::linter::RelativePath;
use std::collections::BTreeSet;

/// Selected missing inputs classified from the sealed request by the application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedMissingInputs {
    /// Requested paths observed absent by the sealed context; they have no source or metadata.
    pub selected_missing: BTreeSet<RelativePath>,
}
