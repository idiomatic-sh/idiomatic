//! The scope namespace alone issues production owner-creation authority.
use crate::application::file_dag::relations::types::ReuseEligibility;

/// A permit's private field prevents unrelated application code from creating a
/// RelationOwner directly. Only the outer scope constructor mints it; the owner
/// reads the reuse eligibility it carries.
pub(crate) struct ScopePermit {
    pub(super) reuse: ReuseEligibility,
}

impl From<ScopePermit> for ReuseEligibility {
    fn from(permit: ScopePermit) -> Self {
        permit.reuse
    }
}
