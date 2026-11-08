//! Worker-local lease capabilities wrap thread-safe immutable publication records.
use crate::application::file_dag::derived::ledger::types::ResourceLease;
use crate::application::file_dag::relations::publication::types::RelationResult;
use crate::application::file_dag::relations::types::NormalizationCharge;
use std::{cell::RefCell, sync::Arc};
#[derive(Debug)]
struct RequiredLeases {
    _initial: Arc<ResourceLease>,
    normalized: RefCell<Option<Arc<ResourceLease>>>,
    normalized_in_initial: NormalizationCharge,
}
/// A publication of a relation result the owner leased: its record, pinned for its holder.
#[derive(Clone, Debug)]
pub struct PublishedInput {
    record: Arc<RelationResult>,
    leases: Arc<RequiredLeases>,
}
impl PublishedInput {
    pub(crate) fn new(
        record: Arc<RelationResult>,
        initial: Arc<ResourceLease>,
    ) -> Self {
        let normalized_in_initial = if record.normalized.get().is_some() {
            NormalizationCharge::Charged
        } else {
            NormalizationCharge::Uncharged
        };
        Self {
            record,
            leases: Arc::new(RequiredLeases {
                _initial: initial,
                normalized: RefCell::new(None),
                normalized_in_initial,
            }),
        }
    }
    /// The published record this input leases, borrowed for reading.
    pub fn as_ref(&self) -> &RelationResult {
        &self.record
    }
    /// The published record.
    #[cfg(any(test, feature = "fixtures"))]
    pub fn record(&self) -> &Arc<RelationResult> {
        &self.record
    }
    pub(crate) fn normalization_charged(&self) -> NormalizationCharge {
        if self.leases.normalized_in_initial == NormalizationCharge::Charged
            || self.leases.normalized.borrow().is_some()
        {
            NormalizationCharge::Charged
        } else {
            NormalizationCharge::Uncharged
        }
    }
    pub(crate) fn hold_normalization(
        &self,
        lease: Arc<ResourceLease>,
    ) {
        *self.leases.normalized.borrow_mut() = Some(lease);
    }
    /// Whether `other` leases the same record.
    #[cfg(any(test, feature = "fixtures"))]
    pub fn same_record(&self, other: &Self) -> crate::application::file_dag::relations::types::Sameness {
        if Arc::ptr_eq(&self.record, &other.record) {
            crate::application::file_dag::relations::types::Sameness::Same
        } else {
            crate::application::file_dag::relations::types::Sameness::Different
        }
    }
}
impl std::ops::Deref for PublishedInput {
    type Target = RelationResult;
    fn deref(&self) -> &Self::Target {
        &self.record
    }
}
