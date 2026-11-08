//! Messages of the invariants the ledger asserts.

pub(super) const LIVE_BACKING: &str = "live backing for resource lease";
#[cfg(test)]
pub(super) const VIEW_BACKING_IN_LEASE: &str = "the actual view backing belongs to this lease";
#[cfg(test)]
pub(super) const VIEW_BACKING_LIVE: &str = "a borrowed view has a live ledger entry";
#[cfg(test)]
pub(super) const VIEW_REQUIRED_LIVE: &str = "a borrowed view remains required-live";
