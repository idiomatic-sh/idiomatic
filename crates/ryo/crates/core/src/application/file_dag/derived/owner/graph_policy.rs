//! Graph priority is an admission policy of the one resource owner.
use super::super::graph_cache::service::room;
use super::super::graph_cache::types::reclamation::value::index_payload;
use super::*;
impl DerivedOwner {
    pub(in crate::application::file_dag::derived) fn prepare_graph_room(
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
            .saturating_sub(super::super::selection::MAX_RETAINED_POSITION_CAPACITY);
        drop(ledger);
        if room(cache, cost).is_some() && required_units == 0 {
            return Ok(true);
        }
        let Some(total) = cache.bytes.checked_add(cost) else {
            return Ok(false);
        };
        let required = total.saturating_sub(cache.limit);
        let (available, pinned) = self.graph_reclaim_credit(cache, protected)?;
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
        if room(cache, cost).is_some() {
            return Ok(true);
        }
        let required = cache
            .bytes
            .checked_add(cost)
            .and_then(|total| total.checked_sub(cache.limit))
            .ok_or(OperatingError::ShapeMismatch)?;
        let mut remaining = required;
        let mut released = 0_usize;
        let mut indexes = 0_u64;
        cache.entries.retain(|entry| {
            if remaining != 0
                && Arc::strong_count(&entry.index) == 1
                && let Some(bytes) = index_payload(entry)
            {
                remaining = remaining.saturating_sub(bytes);
                released += bytes;
                indexes += 1;
                return false;
            }
            true
        });
        cache.bytes -= released;
        cache.graph_counts.reclaimed_indexes += indexes;
        cache.graph_counts.reclaimed_bytes += released as u64;
        Ok(room(cache, cost).is_some())
    }
    fn graph_reclaim_credit(
        &self,
        cache: &CacheBackend,
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
        let mut leases = candidates
            .iter()
            .filter_map(|entry| entry.retained.as_ref().map(|retained| &retained._lease))
            .collect::<Vec<_>>();
        let mut metadata = 0usize;
        let mut pinned = false;
        for entry in &cache.entries {
            let aliases = candidates.iter().filter(|candidate| candidate.retained.as_ref().is_some_and(|retained| matches!(retained.storage.as_ref(), DerivedStorage::EqualityShared(index) if Arc::ptr_eq(index, &entry.index)))).count();
            if Arc::strong_count(&entry.index) == 1 + aliases {
                if let Some(lease) = &entry.payload_lease {
                    leases.push(lease);
                }
                metadata = metadata
                    .checked_add(
                        entry.columns.capacity()
                            * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>(),
                    )
                    .ok_or(OperatingError::ShapeMismatch)?;
            } else {
                pinned = true;
            }
        }
        let Some((physical, units)) = self.ledger.borrow().optional_release_credit(&leases)? else {
            return Ok((None, pinned));
        };
        Ok((
            Some((
                physical
                    .checked_add(metadata)
                    .ok_or(OperatingError::ShapeMismatch)?,
                units,
            )),
            pinned,
        ))
    }
}
