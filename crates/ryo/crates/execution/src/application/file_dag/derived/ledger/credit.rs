//! Read-only preflight of optional handles; it never mutates ownership or payloads.
use super::*;
use std::sync::Arc;
impl Ledger {
    /// A live required/node consumer is not dormant optional payload, even
    /// when a different representation owns the same physical allocation.
    pub(in crate::application::file_dag::derived) fn optional_payload_only(
        &self,
        backings: &[Backing],
    ) -> bool {
        backings.iter().all(|backing| {
            self.entries
                .get(&backing.key)
                .is_some_and(|entry| entry.required == 0 && entry.node_optional == 0)
        })
    }
    pub(in crate::application::file_dag::derived) fn optional_release_credit(
        &self,
        leases: &[&Arc<ResourceLease>],
    ) -> Result<Option<(usize, usize)>, OperatingError> {
        let mut handles = HashMap::<usize, (&Arc<ResourceLease>, usize)>::new();
        if handles.try_reserve(leases.len()).is_err() {
            return Ok(None);
        }
        let mut keys = 0usize;
        for lease in leases {
            if !std::ptr::eq(self, lease.ledger.as_ptr()) || lease.class != LeaseClass::Optional {
                return Err(OperatingError::ShapeMismatch);
            }
            let entry = handles
                .entry(Arc::as_ptr(lease) as usize)
                .or_insert((lease, 0));
            entry.1 = entry
                .1
                .checked_add(1)
                .ok_or(OperatingError::ShapeMismatch)?;
            keys = keys
                .checked_add(lease.keys.len())
                .ok_or(OperatingError::ShapeMismatch)?;
        }
        let mut references = HashMap::<Key, usize>::new();
        if references.try_reserve(keys).is_err() {
            return Ok(None);
        }
        for (lease, dropping_handles) in handles.values() {
            if Arc::strong_count(lease) != *dropping_handles {
                continue;
            }
            for key in &lease.keys {
                let count = references.entry(*key).or_default();
                *count = count.checked_add(1).ok_or(OperatingError::ShapeMismatch)?;
            }
        }
        let mut bytes = 0usize;
        let mut units = 0usize;
        for (key, releasing) in references {
            let entry = self
                .entries
                .get(&key)
                .ok_or(OperatingError::ShapeMismatch)?;
            if releasing > entry.optional {
                return Err(OperatingError::ShapeMismatch);
            }
            if releasing == entry.optional {
                units = units
                    .checked_add(entry.retention_units)
                    .ok_or(OperatingError::ShapeMismatch)?;
            }
            if releasing == entry.optional && entry.required == 0 && entry.node_optional == 0 {
                bytes = bytes
                    .checked_add(entry.bytes)
                    .ok_or(OperatingError::ShapeMismatch)?;
            }
        }
        Ok(Some((bytes, units)))
    }
}
