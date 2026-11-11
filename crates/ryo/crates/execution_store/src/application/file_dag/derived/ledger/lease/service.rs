//! The `lease` application operation.

use super::super::{
    constants,
    types::{Backing, BackingKey, ByteCount, HoldCount, LeaseClass, Measured, SlotCount},
};
use crate::application::file_dag::derived::ledger::service::ResourceLease;
use rustc_hash::FxHashMap;
use ryo_execution_domain::domain::operating::errors::OperatingError;

#[cfg(test)]
impl ResourceLease {
    pub(crate) fn assert_required(&self, backings: &[Backing]) {
        assert_eq!(self.class, LeaseClass::Required);
        let ledger = self.ledger.borrow();
        for backing in backings {
            assert!(
                self.keys.contains(&backing.key),
                "{}",
                constants::VIEW_BACKING_IN_LEASE
            );
            let entry = ledger
                .entries
                .get(&backing.key)
                .expect(constants::VIEW_BACKING_LIVE);
            assert!(
                entry.required > HoldCount(0),
                "{}",
                constants::VIEW_REQUIRED_LIVE
            );
            assert_eq!(entry.bytes, backing.bytes);
            assert_eq!(entry.retention_units, backing.retention_units);
        }
    }
}

impl Drop for ResourceLease {
    #[expect(
        clippy::expect_used,
        reason = "a lease holds every key it charged until this drop, its only release: a missing entry is a broken ledger, and a drop cannot return the error"
    )]
    fn drop(&mut self) {
        let mut state = self.ledger.borrow_mut();
        for key in &self.keys {
            let entry = state.entries.get_mut(key).expect(constants::LIVE_BACKING);
            match self.class {
                LeaseClass::Required => entry.required -= HoldCount(1),
                LeaseClass::Optional => entry.optional -= HoldCount(1),
            }
            let bytes = entry.bytes;
            let units = entry.retention_units;
            let class_live = match self.class {
                LeaseClass::Required => entry.required != HoldCount(0),
                LeaseClass::Optional => entry.optional != HoldCount(0),
            };
            let required_live = entry.required != HoldCount(0);
            let optional_live = entry.optional != HoldCount(0);
            if !class_live {
                match self.class {
                    LeaseClass::Required => state.required_bytes -= bytes,
                    LeaseClass::Optional => {
                        state.view_optional_bytes -= bytes;
                        state.optional_units -= units;
                    }
                }
                if matches!(self.class, LeaseClass::Optional) && !optional_live {
                    state.optional_bytes -= bytes;
                }
                if (matches!(self.class, LeaseClass::Required) && optional_live)
                    || (matches!(self.class, LeaseClass::Optional)
                        && required_live
                        && !optional_live)
                {
                    state.overlap_bytes -= bytes;
                }
            }
            if !required_live && !optional_live {
                state.unique_bytes -= bytes;
                state.entries.remove(key);
            }
        }
        state.lease_metadata -= self.metadata_bytes;
    }
}

/// The distinct backings of a request, each with its size; a key that disagrees with itself refuses.
pub(crate) fn measured(backings: &[Backing]) -> Result<Vec<Measured>, OperatingError> {
    let mut seen = FxHashMap::<BackingKey, Backing>::with_capacity_and_hasher(
        backings.len(),
        rustc_hash::FxBuildHasher,
    );
    for backing in backings {
        if seen.insert(backing.key, *backing).is_some_and(|prior| {
            prior.bytes != backing.bytes || prior.retention_units != backing.retention_units
        }) {
            return Err(OperatingError::ShapeMismatch);
        }
    }
    Ok(seen
        .into_values()
        .map(|backing| Measured {
            key: backing.key,
            bytes: backing.bytes,
            units: backing.retention_units,
        })
        .collect())
}

pub(crate) fn lease_bytes(key_capacity: SlotCount) -> Result<ByteCount, OperatingError> {
    size_of::<ResourceLease>()
        .checked_add(2 * size_of::<usize>()) // outer Arc allocation header
        .and_then(|base| {
            key_capacity
                .0
                .checked_mul(size_of::<BackingKey>())
                .and_then(|keys| base.checked_add(keys))
        })
        .map(ByteCount)
        .ok_or_else(|| OperatingError::ShapeMismatch)
}

pub(crate) fn append_backing(
    backings: &mut Vec<Backing>,
    next: Backing,
) -> Result<(), OperatingError> {
    if let Some(old) = backings.iter().find(|old| old.key == next.key) {
        if old.bytes != next.bytes || old.retention_units != next.retention_units {
            return Err(OperatingError::ShapeMismatch);
        }
    } else {
        backings.push(next);
    }
    Ok(())
}
