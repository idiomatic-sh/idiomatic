//! Touched population release and reusable metadata lifetime.
use super::*;
impl DerivedOwner {
    pub(in crate::application::file_dag) fn pin(&self, population: PopulationId) {
        let mut state = self.state.borrow_mut();
        *state.pins.entry(population).or_default() += 1;
    }
    pub(in crate::application::file_dag) fn release(&self, population: PopulationId) {
        let mut state = self.state.borrow_mut();
        let Some(count) = state.pins.get_mut(&population) else {
            return;
        };
        *count = count.saturating_sub(1);
        if *count != 0 {
            return;
        }
        state.pins.remove(&population);
        let mut released = 0;
        let mut released_overlap = 0;
        let mut released_views = 0;
        let mut released_backend = 0;
        // Keep this population's reverse list for later re-pins. Taking its
        // buffer releases the map borrow without allocating or scanning other populations.
        if let Some(keys) = state.dependents.get_mut(&population).map(std::mem::take) {
            for key in &keys {
                state.released_entries += 1;
                let still_required = key.populations[..usize::from(key.len)]
                    .iter()
                    .any(|id| state.pins.contains_key(id));
                if still_required {
                    continue;
                }
                if let Some(entry) = state.entries.get_mut(key) {
                    if entry.required.take().is_some() {
                        released += entry.working;
                        released_backend += entry.backend;
                        released_overlap += usize::from(entry.retained.is_some()) * entry.bytes;
                        released_views += 1;
                    }
                }
            }
            *state
                .dependents
                .get_mut(&population)
                .expect("known population") = keys;
        }
        state.required_bytes = state.required_bytes.saturating_sub(released);
        state.overlap_bytes = state.overlap_bytes.saturating_sub(released_overlap);
        state.required_views = state.required_views.saturating_sub(released_views);
        state.required_backend_bytes = state
            .required_backend_bytes
            .saturating_sub(released_backend);
    }
    /// Population mutation ends every identity and every owned view before provider refill.
    pub(in crate::application::file_dag) fn clear(&self) {
        self.file_prepared.set(false);
        let mut state = self.state.borrow_mut();
        state.entries.clear();
        state.next_admission = 0;
        state.pins.clear();
        let mut recycled = std::mem::take(&mut state.spare_dependencies);
        for (_, mut keys) in state.dependents.drain() {
            keys.clear();
            recycled.push(keys);
        }
        state.spare_dependencies = recycled;
        state.required_bytes = 0;
        state.retained_bytes = 0;
        state.retained_predicates = 0;
        state.overlap_bytes = 0;
        state.required_views = 0;
        state.required_backend_bytes = 0;
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
