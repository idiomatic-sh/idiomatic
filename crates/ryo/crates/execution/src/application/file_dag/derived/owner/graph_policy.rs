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
            .saturating_sub(crate::application::file_dag::derived::owner::selection::MAX_RETAINED_POSITION_CAPACITY);
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
