//! Graph priority is an admission policy of the one resource owner.
use super::*;
use crate::application::file_dag::derived::ledger::types::Backing;
use crate::application::file_dag::derived::ledger::types::{ByteCount, UnitCount};
use crate::application::file_dag::derived::owner::graphs::service::room;
impl DerivedOwner {
    pub(crate) fn prepare_graph_room(
        &self,
        cache: &mut CacheBackend,
        cost: ByteCount,
        protected: &[super::super::ledger::types::Backing],
    ) -> Result<Room, OperatingError> {
        let ledger = self.ledger.borrow();
        let extra_units = ledger.incremental_optional_units(protected)?;
        let required_units = ledger
            .snapshot()
            .optional_units
            .0
            .checked_add(extra_units.0)
            .ok_or(OperatingError::ShapeMismatch)?
            .saturating_sub(crate::application::file_dag::derived::owner::cache::types::MAX_RETAINED_POSITION_CAPACITY);
        drop(ledger);
        if room(cache, cost).is_some() && required_units == 0 {
            return Ok(Room::Fits);
        }
        let Some(total) = cache.bytes.0.checked_add(cost.0) else {
            return Ok(Room::Full);
        };
        let required = total.saturating_sub(cache.limit.0);
        let outlook = self.graph_reclaim_credit(protected)?;
        let Some(credit) = outlook.available else {
            return Ok(Room::Full);
        };
        if credit.bytes.0 < required || credit.units.0 < required_units {
            cache.graph_counts.pinned_refusals.0 += u64::from(outlook.pinned == PinState::Pinned);
            return Ok(Room::Full);
        }
        // No optional payload is removed until the complete admission has
        // proved enough credit, including shared-owner aliases exactly once.
        self.reclaim_optional_preserving(cache, cost, protected, extra_units);
        Ok(Room::from(room(cache, cost).is_some()))
    }

    fn graph_reclaim_credit(
        &self,
        protected: &[super::super::ledger::types::Backing],
    ) -> Result<ReclaimOutlook, OperatingError> {
        let state = self.state.borrow();
        let ledger = self.ledger.borrow();
        let candidates = state
            .entries
            .values()
            .filter(|entry| {
                entry.required.is_none()
                    && ledger.optional_payload_only(&entry.backings)
                        == crate::application::file_dag::derived::ledger::types::Dormancy::Dormant
                    && entry
                        .retained
                        .as_ref()
                        .is_some_and(|retained| Arc::strong_count(&retained.storage) == 1)
                    && !entry
                        .backings
                        .iter()
                        .any(|backing| protected.iter().any(|needed| needed.key == backing.key))
            })
            .collect::<Vec<_>>();
        drop(ledger);
        let leases = candidates
            .iter()
            .filter_map(|entry| entry.retained.as_ref().map(|retained| &retained._lease))
            .collect::<Vec<_>>();
        let pinned = if state.entries.values().any(|entry| {
            entry.retained.as_ref().is_some_and(|retained| {
                entry.required.is_some() || Arc::strong_count(&retained.storage) > 1
            })
        }) {
            PinState::Pinned
        } else {
            PinState::Free
        };
        let Some(credit) = self.ledger.borrow().optional_release_credit(&leases)? else {
            return Ok(ReclaimOutlook {
                available: None,
                pinned,
            });
        };
        Ok(ReclaimOutlook {
            available: Some(credit),
            pinned,
        })
    }
}

impl DerivedOwner {
    pub(super) fn reclaim_optional_preserving(
        &self,
        cache: &mut CacheBackend,
        cost: ByteCount,
        protected: &[Backing],
        extra_units: UnitCount,
    ) {
        let mut state = self.state.borrow_mut();
        let ledger = self.ledger.borrow();
        let mut candidates = state
            .entries
            .iter()
            .filter_map(|(key, entry)| {
                (entry.required.is_none()
                    && ledger.optional_payload_only(&entry.backings)
                        == crate::application::file_dag::derived::ledger::types::Dormancy::Dormant
                    && entry
                        .retained
                        .as_ref()
                        .is_some_and(|retained| Arc::strong_count(&retained.storage) == 1)
                    && !entry
                        .backings
                        .iter()
                        .any(|backing| protected.iter().any(|needed| needed.key == backing.key)))
                .then(|| (entry.admission, key.clone()))
            })
            .collect::<Vec<_>>();
        drop(ledger);
        candidates.sort_unstable_by_key(|candidate| candidate.0);
        let mut released = ByteCount(0);
        let mut released_indexes = 0;
        for (_, key) in candidates {
            if cache
                .bytes
                .0
                .checked_add(cost.0)
                .is_some_and(|bytes| bytes <= cache.limit.0)
                && self
                    .ledger
                    .borrow()
                    .snapshot()
                    .optional_units
                    .0
                    .checked_add(extra_units.0)
                    .is_some_and(|units| units <= crate::application::file_dag::derived::owner::cache::types::MAX_RETAINED_POSITION_CAPACITY)
            {
                break;
            }
            let before = self.ledger.borrow().snapshot().view_optional_bytes.0;
            let Some(entry) = state.entries.get_mut(&key) else {
                continue;
            };
            let removed = entry.retained.take();
            released += entry.bytes;
            released_indexes += u64::from(entry._plan.kind == DerivedKind::Equality);
            // Drop the optional lease before observing the physical union. A
            // graph or publication alias may still own some of these backings.
            drop(removed);
            let after = self.ledger.borrow().snapshot().view_optional_bytes.0;
            let physical = before.saturating_sub(after);
            cache.bytes.0 = cache.bytes.0.saturating_sub(physical);
            cache.graph_counts.reclaimed_bytes.0 += physical as u64;
        }
        state.retained_bytes = ByteCount(state.retained_bytes.0.saturating_sub(released.0));
        cache.graph_counts.reclaimed_indexes.0 += released_indexes;
    }
}
