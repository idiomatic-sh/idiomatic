//! Read-only preflight of optional handles; it never mutates ownership or payloads.
use super::types::{
    Backing, BackingKey, ByteCount, Credit, Dormancy, HoldCount, LeaseClass, UnitCount,
};
use crate::application::file_dag::derived::ledger::service::Ledger;
use crate::application::file_dag::derived::ledger::service::ResourceLease;
use rustc_hash::FxHashMap;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::sync::Arc;

/// A handle being released, with how many of its clones are being released with it.
struct Dropping<'a> {
    lease: &'a Arc<ResourceLease>,
    handles: HoldCount,
}

impl Ledger {
    /// A live required consumer is not dormant optional payload, even
    /// when a different representation owns the same physical allocation.
    pub(crate) fn optional_payload_only(&self, backings: &[Backing]) -> Dormancy {
        if backings.iter().all(|backing| {
            self.entries
                .get(&backing.key)
                .is_some_and(|entry| entry.required == HoldCount(0))
        }) {
            Dormancy::Dormant
        } else {
            Dormancy::Live
        }
    }
    pub(crate) fn optional_release_credit(
        &self,
        leases: &[&Arc<ResourceLease>],
    ) -> Result<Option<Credit>, OperatingError> {
        let mut handles = FxHashMap::<usize, Dropping<'_>>::default();
        if handles.try_reserve(leases.len()).is_err() {
            return Ok(None);
        }
        let mut keys = 0usize;
        for lease in leases {
            if !std::ptr::eq(self, lease.ledger.as_ptr()) || lease.class != LeaseClass::Optional {
                return Err(OperatingError::ShapeMismatch);
            }
            let entry = handles
                .entry(Arc::as_ptr(lease).addr())
                .or_insert(Dropping {
                    lease,
                    handles: HoldCount(0),
                });
            entry.handles = entry
                .handles
                .checked_add(HoldCount(1))
                .ok_or_else(|| OperatingError::ShapeMismatch)?;
            keys = keys
                .checked_add(lease.keys.len())
                .ok_or_else(|| OperatingError::ShapeMismatch)?;
        }
        let mut references = FxHashMap::<BackingKey, HoldCount>::default();
        if references.try_reserve(keys).is_err() {
            return Ok(None);
        }
        for dropping in handles.values() {
            if Arc::strong_count(dropping.lease) != dropping.handles.0 {
                continue;
            }
            for key in &dropping.lease.keys {
                let count = references.entry(*key).or_default();
                *count = count
                    .checked_add(HoldCount(1))
                    .ok_or_else(|| OperatingError::ShapeMismatch)?;
            }
        }
        let mut bytes = ByteCount(0);
        let mut units = UnitCount(0);
        for (key, releasing) in references {
            let entry = self
                .entries
                .get(&key)
                .ok_or_else(|| OperatingError::ShapeMismatch)?;
            if releasing > entry.optional {
                return Err(OperatingError::ShapeMismatch);
            }
            if releasing == entry.optional {
                units = units
                    .checked_add(entry.retention_units)
                    .ok_or_else(|| OperatingError::ShapeMismatch)?;
            }
            if releasing == entry.optional && entry.required == HoldCount(0) {
                bytes = bytes
                    .checked_add(entry.bytes)
                    .ok_or_else(|| OperatingError::ShapeMismatch)?;
            }
        }
        Ok(Some(Credit { bytes, units }))
    }
}
