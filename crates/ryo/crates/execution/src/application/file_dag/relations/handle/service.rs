//! Worker-local lease capabilities wrap thread-safe immutable publication records.
use crate::application::file_dag::derived::ledger::types::ResourceLease;
use crate::application::file_dag::derived::owner::acquisition::admission::service::ResourceDependency;
use crate::application::file_dag::relations::publication::types::RelationResult;
use std::{cell::RefCell, sync::Arc};
#[derive(Debug)]
struct RequiredLeases {
    _initial: Arc<ResourceLease>,
    normalized: RefCell<Option<Arc<ResourceLease>>>,
    normalized_in_initial: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct PublishedInput {
    record: Arc<RelationResult>,
    leases: Arc<RequiredLeases>,
    /// Keep derived backing leases alive until the final publication consumer drops.
    _resources: Option<Arc<[ResourceDependency]>>,
}
impl PublishedInput {
    pub(in crate::application::file_dag::relations) fn new(
        record: Arc<RelationResult>,
        initial: Arc<ResourceLease>,
        resources: Option<Arc<[ResourceDependency]>>,
    ) -> Self {
        let normalized_in_initial = record.normalized.get().is_some();
        Self {
            record,
            _resources: resources,
            leases: Arc::new(RequiredLeases {
                _initial: initial,
                normalized: RefCell::new(None),
                normalized_in_initial,
            }),
        }
    }
    pub(crate) fn as_ref(&self) -> &RelationResult {
        &self.record
    }
    pub(in crate::application::file_dag::relations) fn record(&self) -> &Arc<RelationResult> {
        &self.record
    }
    pub(in crate::application::file_dag::relations) fn normalization_charged(&self) -> bool {
        self.leases.normalized_in_initial || self.leases.normalized.borrow().is_some()
    }
    pub(in crate::application::file_dag::relations) fn hold_normalization(
        &self,
        lease: Arc<ResourceLease>,
    ) {
        *self.leases.normalized.borrow_mut() = Some(lease);
    }
    #[cfg(test)]
    pub(crate) fn same_record(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.record, &other.record)
    }
}
impl std::ops::Deref for PublishedInput {
    type Target = RelationResult;
    fn deref(&self) -> &Self::Target {
        &self.record
    }
}
