//! Touched population release and reusable metadata lifetime.
use super::*;
impl DerivedOwner {
    pub(crate) fn pin(&self, population: PopulationId) {
        let mut state = self.state.borrow_mut();
        *state.pins.entry(population).or_default() += 1_usize;
    }
    /// Releases every view of `population`.
    pub fn release(&self, population: PopulationId) {
        let mut state = self.state.borrow_mut();
        let Some(count) = state.pins.get_mut(&population) else {
            return;
        };
        count.0 = count.0.saturating_sub(1);
        if count.0 != 0 {
            return;
        }
        state.pins.remove(&population);
        let mut released = ByteCount(0);
        let mut released_overlap = ByteCount(0);
        let mut released_views = 0_usize;
        let mut released_backend = ByteCount(0);
        // Keep this population's reverse list for later re-pins. Taking its
        // buffer releases the map borrow without allocating or scanning other populations.
        if let Some(keys) = state.dependents.get_mut(&population).map(std::mem::take) {
            for key in &keys {
                state.released_entries += 1_u64;
                let still_required = key.populations[..usize::from(key.len.0)]
                    .iter()
                    .any(|id| state.pins.contains_key(id));
                if still_required {
                    continue;
                }
                if let Some(entry) = state.entries.get_mut(key) {
                    if entry.required.take().is_some() {
                        released += entry.working;
                        released_backend += entry.backend;
                        if entry.retained.is_some() {
                            released_overlap += entry.bytes;
                        }
                        released_views += 1_usize;
                    }
                }
            }
            *state
                .dependents
                .get_mut(&population)
                .expect(constants::EXPECT_KNOWN_POPULATION) = keys;
        }
        state.required_bytes = ByteCount(state.required_bytes.0.saturating_sub(released.0));
        state.overlap_bytes = ByteCount(state.overlap_bytes.0.saturating_sub(released_overlap.0));
        state.required_views = ViewCount(state.required_views.0.saturating_sub(released_views));
        state.required_backend_bytes = ByteCount(
            state
                .required_backend_bytes
                .0
                .saturating_sub(released_backend.0),
        );
    }
    /// Population mutation ends every identity and every owned view before provider refill.
    pub(crate) fn clear(&self) {
        // A file owner is prepared again for its next file; the project owner stays the project's.
        if self.file_prepared.get() == FilePreparation::Prepared {
            self.file_prepared.set(FilePreparation::Unprepared);
        }
        let mut state = self.state.borrow_mut();
        state.entries.clear();
        state.next_admission = AdmissionOrder(0);
        state.pins.clear();
        let mut recycled = std::mem::take(&mut state.spare_dependencies);
        for (_, mut keys) in state.dependents.drain() {
            keys.clear();
            recycled.push(keys);
        }
        state.spare_dependencies = recycled;
        state.required_bytes = ByteCount(0);
        state.retained_bytes = ByteCount(0);
        state.overlap_bytes = ByteCount(0);
        state.required_views = ViewCount(0);
        state.required_backend_bytes = ByteCount(0);
        drop(state);
        // Population mutation invalidates every backend identity, including
        // adjacency and memo records, not only the common view-key table.
        let backend = self.backend.borrow_mut().take();
        drop(backend);
    }
}

#[cfg(test)]
mod graph;
#[cfg(test)]
mod index;
