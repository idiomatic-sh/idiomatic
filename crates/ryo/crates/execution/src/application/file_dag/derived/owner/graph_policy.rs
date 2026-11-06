//! Graph priority is an admission policy of the one resource owner.
use super::*;
use crate::application::file_dag::derived::owner::graphs::service::room;
impl DerivedOwner {
    pub(in crate::application::file_dag::derived::owner) fn prepare_graph_room(
        &self,
        cache: &mut CacheBackend,
        cost: usize,
        protected: &[super::super::ledger::Backing],
    ) -> Result<bool, OperatingError> {
        let ledger = self.ledger.borrow();
        let extra_units = ledger.incremental_optional_units(protected)?;
        let required_units = ledger
            .snapshot()
            .optional_units
            .checked_add(extra_units)
            .ok_or(OperatingError::ShapeMismatch)?
            .saturating_sub(crate::application::file_dag::derived::owner::selection::types::MAX_RETAINED_POSITION_CAPACITY);
        drop(ledger);
        if room(cache, cost).is_some() && required_units == 0 {
            return Ok(true);
        }
        let Some(total) = cache.bytes.checked_add(cost) else {
            return Ok(false);
        };
        let required = total.saturating_sub(cache.limit);
        let (available, pinned) = self.graph_reclaim_credit(protected)?;
        let Some((bytes, units)) = available else {
            return Ok(false);
        };
        if bytes < required || units < required_units {
            cache.graph_counts.pinned_refusals += u64::from(pinned);
            return Ok(false);
        }
        // No optional payload is removed until the complete admission has
        // proved enough credit, including shared-owner aliases exactly once.
        self.reclaim_optional_preserving(cache, cost, protected, extra_units);
        Ok(room(cache, cost).is_some())
    }

    fn graph_reclaim_credit(
        &self,
        protected: &[super::super::ledger::Backing],
    ) -> Result<(Option<(usize, usize)>, bool), OperatingError> {
        let state = self.state.borrow();
        let ledger = self.ledger.borrow();
        let candidates = state
            .entries
            .values()
            .filter(|entry| {
                entry.required.is_none()
                    && ledger.optional_payload_only(&entry.backings)
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
        let pinned = state.entries.values().any(|entry| {
            entry.retained.as_ref().is_some_and(|retained| {
                entry.required.is_some() || Arc::strong_count(&retained.storage) > 1
            })
        });
        let Some((physical, units)) = self.ledger.borrow().optional_release_credit(&leases)? else {
            return Ok((None, pinned));
        };
        Ok((Some((physical, units)), pinned))
    }
}

impl DerivedOwner {
    pub(super) fn reclaim_optional_preserving(
        &self,
        cache: &mut CacheBackend,
        cost: usize,
        protected: &[super::ledger::Backing],
        extra_units: usize,
    ) {
        let mut state = self.state.borrow_mut();
        let ledger = self.ledger.borrow();
        let mut candidates = state
            .entries
            .iter()
            .filter_map(|(key, entry)| {
                (entry.required.is_none()
                    && ledger.optional_payload_only(&entry.backings)
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
        let mut released = 0;
        let mut released_predicates = 0;
        let mut released_indexes = 0;
        for (_, key) in candidates {
            if cache
                .bytes
                .checked_add(cost)
                .is_some_and(|bytes| bytes <= cache.limit)
                && self
                    .ledger
                    .borrow()
                    .snapshot()
                    .optional_units
                    .checked_add(extra_units)
                    .is_some_and(|units| units <= crate::application::file_dag::derived::owner::selection::types::MAX_RETAINED_POSITION_CAPACITY)
            {
                break;
            }
            let before = self.ledger.borrow().snapshot().view_optional_bytes;
            let Some(entry) = state.entries.get_mut(&key) else {
                continue;
            };
            let removed = entry.retained.take();
            released += entry.bytes;
            released_predicates += usize::from(entry._plan.kind == DerivedKind::PredicateMatches);
            released_indexes += u64::from(entry._plan.kind == DerivedKind::Equality);
            // Drop the optional lease before observing the physical union. A
            // graph or publication alias may still own some of these backings.
            drop(removed);
            let after = self.ledger.borrow().snapshot().view_optional_bytes;
            let physical = before.saturating_sub(after);
            cache.bytes = cache.bytes.saturating_sub(physical);
            cache.graph_counts.reclaimed_bytes += physical as u64;
        }
        state.retained_bytes = state.retained_bytes.saturating_sub(released);
        state.retained_predicates = state
            .retained_predicates
            .saturating_sub(released_predicates);
        cache.graph_counts.reclaimed_selections += released_predicates as u64;
        cache.graph_counts.reclaimed_indexes += released_indexes;
    }
}
