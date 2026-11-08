//! Accounting for physical payloads held by derived resource handles.
//! Admission policy, cache selection and scheduling stay with the caller.
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::{cell::RefCell, rc::Rc};
use rustc_hash::FxHashMap;

mod arithmetic;
pub(crate) mod capacity;
mod constants;
mod credit;
pub(crate) mod lease;
mod snapshot;
#[cfg(test)]
mod test;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
use types::{BackingCount, BackingTag};
use types::{
    Backing, ByteCount, HoldCount, LeaseClass, Ledger, LedgerEntry, ResourceLease, SlotCount,
    UnitCount,
};

impl Default for Ledger {
    fn default() -> Self {
        let mut ledger = Self {
            entries: FxHashMap::default(),
            required_bytes: ByteCount(0),
            optional_bytes: ByteCount(0),
            view_optional_bytes: ByteCount(0),
            optional_units: UnitCount(0),
            overlap_bytes: ByteCount(0),
            unique_bytes: ByteCount(0),
            lease_metadata: ByteCount(0),
            peak_required_bytes: ByteCount(0),
            peak_optional_bytes: ByteCount(0),
            peak_view_optional_bytes: ByteCount(0),
            peak_optional_units: UnitCount(0),
            peak_overlap_bytes: ByteCount(0),
            peak_unique_bytes: ByteCount(0),
            peak_metadata_bytes: ByteCount(0),
        };
        ledger.note_peaks();
        ledger
    }
}

impl Ledger {
    #[cfg(any(test, feature = "fixtures"))]
    pub(super) fn backing_count(&self, tag: BackingTag) -> BackingCount {
        BackingCount(self.entries.keys().filter(|key| key.tag == tag).count())
    }
    /// Bytes newly retained by the view optional class. A required-only
    /// backing is charged because a view lease may outlive it.
    pub(super) fn incremental_optional_bytes(
        &self,
        backings: &[Backing],
    ) -> Result<ByteCount, OperatingError> {
        let mut additional = ByteCount(0);
        for each in lease::measured(backings)? {
            match self.entries.get(&each.key) {
                Some(entry) if entry.bytes != each.bytes || entry.retention_units != each.units => {
                    return Err(OperatingError::ShapeMismatch);
                }
                Some(entry) if entry.optional == HoldCount(0) => {
                    additional = additional
                        .checked_add(each.bytes)
                        .ok_or(OperatingError::ShapeMismatch)?
                }
                Some(_) => {}
                None => {
                    additional = additional
                        .checked_add(each.bytes)
                        .ok_or(OperatingError::ShapeMismatch)?
                }
            }
        }
        Ok(additional)
    }

    /// Position capacity newly retained by the optional class, independent of
    /// the byte limit. Required-only aliases still consume this allowance.
    pub(super) fn incremental_optional_units(
        &self,
        backings: &[Backing],
    ) -> Result<UnitCount, OperatingError> {
        let mut additional = UnitCount(0);
        for each in lease::measured(backings)? {
            match self.entries.get(&each.key) {
                Some(entry) if entry.bytes != each.bytes || entry.retention_units != each.units => {
                    return Err(OperatingError::ShapeMismatch);
                }
                Some(entry) if entry.optional != HoldCount(0) => {}
                _ => {
                    additional = additional
                        .checked_add(each.units)
                        .ok_or(OperatingError::ShapeMismatch)?;
                }
            }
        }
        Ok(additional)
    }

    /// Register one handle. The caller may share it through an outer `Arc`;
    /// cloning that `Arc` does not create another ledger lease.
    pub(super) fn acquire(
        ledger: &Rc<RefCell<Self>>,
        backings: &[Backing],
        class: LeaseClass,
    ) -> Result<ResourceLease, OperatingError> {
        let unique = lease::measured(backings)?;
        let mut keys = Vec::with_capacity(unique.len());
        let mut state = ledger.borrow_mut();
        let mut next_required = state.required_bytes;
        let mut next_optional = state.optional_bytes;
        let mut next_view_optional = state.view_optional_bytes;
        let mut next_optional_units = state.optional_units;
        let mut next_overlap = state.overlap_bytes;
        let mut next_unique = state.unique_bytes;
        for each in &unique {
            let old = state.entries.get(&each.key);
            if old.is_some_and(|entry| {
                entry.bytes != each.bytes || entry.retention_units != each.units
            }) {
                return Err(OperatingError::ShapeMismatch);
            }
            let required = old.map_or(HoldCount(0), |entry| entry.required);
            let optional = old.map_or(HoldCount(0), |entry| entry.optional);
            match class {
                LeaseClass::Required => {
                    required
                        .checked_add(HoldCount(1))
                        .ok_or(OperatingError::ShapeMismatch)?;
                    if required == HoldCount(0) {
                        next_required = next_required
                            .checked_add(each.bytes)
                            .ok_or(OperatingError::ShapeMismatch)?;
                        if optional != HoldCount(0) {
                            next_overlap = next_overlap
                                .checked_add(each.bytes)
                                .ok_or(OperatingError::ShapeMismatch)?;
                        }
                    }
                }
                LeaseClass::Optional => {
                    optional
                        .checked_add(HoldCount(1))
                        .ok_or(OperatingError::ShapeMismatch)?;
                    if optional == HoldCount(0) {
                        next_view_optional = next_view_optional
                            .checked_add(each.bytes)
                            .ok_or(OperatingError::ShapeMismatch)?;
                        next_optional_units = next_optional_units
                            .checked_add(each.units)
                            .ok_or(OperatingError::ShapeMismatch)?;
                        next_optional = next_optional
                            .checked_add(each.bytes)
                            .ok_or(OperatingError::ShapeMismatch)?;
                        if required != HoldCount(0) {
                            next_overlap = next_overlap
                                .checked_add(each.bytes)
                                .ok_or(OperatingError::ShapeMismatch)?;
                        }
                    }
                }
            }
            if old.is_none() {
                next_unique = next_unique
                    .checked_add(each.bytes)
                    .ok_or(OperatingError::ShapeMismatch)?;
            }
            keys.push(each.key);
        }
        let lease_bytes = lease::lease_bytes(SlotCount(keys.capacity()))?;
        let next_lease_metadata = state
            .lease_metadata
            .checked_add(lease_bytes)
            .ok_or(OperatingError::ShapeMismatch)?;
        for each in unique {
            let entry = state.entries.entry(each.key).or_insert(LedgerEntry {
                bytes: each.bytes,
                retention_units: each.units,
                required: HoldCount(0),
                optional: HoldCount(0),
            });
            match class {
                LeaseClass::Required => entry.required += HoldCount(1),
                LeaseClass::Optional => entry.optional += HoldCount(1),
            }
        }
        state.required_bytes = next_required;
        state.optional_bytes = next_optional;
        state.view_optional_bytes = next_view_optional;
        state.optional_units = next_optional_units;
        state.overlap_bytes = next_overlap;
        state.unique_bytes = next_unique;
        state.lease_metadata = next_lease_metadata;
        state.note_peaks();
        Ok(ResourceLease {
            ledger: Rc::clone(ledger),
            keys,
            class,
            metadata_bytes: lease_bytes,
        })
    }
}
