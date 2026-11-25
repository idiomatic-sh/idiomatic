//! A file borrows the programs selected for an inventory. Their report addresses
//! differ by the file's ordinal. Overlapping selections retain one binding each.
use std::sync::Arc;

type Job = (usize, usize);

#[derive(Clone)]
struct Selection {
    bases: Arc<[Job]>,
    ordinal: usize,
}

/// Most inventories admit one program set. Only scopes with irregular overlap
/// or store hits need individually addressed jobs.
#[derive(Clone, Default)]
pub(crate) struct FilePrograms {
    common: Option<Selection>,
    individual: Vec<Job>,
}

impl FilePrograms {
    /// Shared program order regardless of the file's ordinal. The source retains
    /// the Arc for the entire visit; irregular overlaps have no shared identity.
    pub(super) fn common_id(&self) -> Option<usize> {
        self.common.as_ref().filter(|_| self.individual.is_empty()).map(|s| s.bases.as_ptr() as usize)
    }

    pub(crate) fn len(&self) -> usize {
        self.common.as_ref().map_or(0, |s| s.bases.len()) + self.individual.len()
    }
    pub(crate) fn is_empty(&self) -> bool { self.len() == 0 }

    pub(crate) fn iter(&self) -> impl Iterator<Item = Job> + Clone + '_ {
        self.common.iter().flat_map(|s|
            s.bases.iter().map(|&(binding, base)| (binding, base + s.ordinal))
        ).chain(self.individual.iter().copied())
    }

    fn materialize(&mut self) -> &mut Vec<Job> {
        if self.common.is_some() {
            let entries = self.iter().collect();
            self.individual = entries;
            self.common = None;
        }
        &mut self.individual
    }

    pub(super) fn lookup(&self, binding: usize) -> Option<usize> {
        if let Some(selection) = &self.common
            && let Ok(i) = selection.bases.binary_search_by_key(&binding, |j| j.0)
        {
            return Some(selection.bases[i].1 + selection.ordinal);
        }
        self.individual.binary_search_by_key(&binding, |j| j.0).ok().map(|i| self.individual[i].1)
    }

    pub(super) fn admit(&mut self, binding: usize, new: impl FnOnce() -> usize) -> usize {
        if self.common.is_some() && let Some(slot) = self.lookup(binding) { return slot; }
        super::admitted_slot(self.materialize(), binding, new)
    }

    pub(super) fn share(&mut self, bases: &Arc<[Job]>, ordinal: usize) {
        if self.is_empty() {
            self.common = Some(Selection { bases: bases.clone(), ordinal });
        } else {
            let jobs = self.materialize();
            jobs.extend(bases.iter().map(|&(binding, base)| (binding, base + ordinal)));
            jobs.sort_unstable_by_key(|x| x.0);
        }
    }

    pub(super) fn clear(&mut self) {
        self.common = None;
        self.individual.clear();
    }

    pub(crate) fn keep(&mut self, mut f: impl FnMut(Job) -> bool) {
        self.materialize().retain(|job| f(*job));
    }

    pub(super) fn keep_programs(
        &mut self, eligible: &impl Fn(usize) -> bool,
        cache: &mut std::collections::HashMap<usize, (Arc<[Job]>, Arc<[Job]>)>,
    ) {
        if let Some(common) = &mut self.common {
            let key = common.bases.as_ptr() as usize;
            let (_, filtered) = cache.entry(key).or_insert_with(|| {
                let bases = &common.bases;
                let kept = if bases.iter().all(|&(b, _)| eligible(b)) { bases.clone() }
                    else { bases.iter().copied().filter(|&(b, _)| eligible(b)).collect() };
                (bases.clone(), kept)
            });
            common.bases = filtered.clone();
        }
        self.individual.retain(|&(b, _)| eligible(b));
    }

    pub(super) fn append(&mut self, incoming: &mut Self) {
        if self.is_empty() {
            std::mem::swap(self, incoming);
        } else if !incoming.is_empty() {
            let jobs = self.materialize();
            jobs.extend(incoming.iter());
            jobs.sort_unstable_by_key(|j| j.0);
            incoming.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check_session::{Schedule, SelectedFiles};

    #[test]
    fn shared_selection_resolves_overlaps_duplicates_and_new_rounds() {
        let mut schedule = Schedule::default();
        let root = Arc::from("root");
        let first = schedule.select_files(&root, &["a".into(), "b".into()]).unwrap();
        assert_eq!(schedule.admit_ids::<Vec<_>>(&first, 0, false), [0, 1]);
        assert_eq!(schedule.admit_ids::<Vec<_>>(&first, 1, false), [2, 3]);
        assert!(schedule.sources.iter().all(|s| s.bindings.is_empty()));
        schedule.finish_selection();
        let a = &schedule.sources[0].bindings;
        let b = &schedule.sources[1].bindings;
        assert!(Arc::ptr_eq(&a.common.as_ref().unwrap().bases, &b.common.as_ref().unwrap().bases));
        assert!(b.iter().eq([(0, 1), (1, 3)]));
        let overlap = Arc::new(SelectedFiles { ids: vec![1, 0, 1], distinct: false });
        assert_eq!(schedule.admit_ids::<Vec<_>>(&overlap, 1, true), [3, 2, 3]);
        schedule.delivered(&[0, 1], true, |id| id == 1);
        assert_eq!(schedule.admit_ids::<Vec<_>>(&overlap, 1, true), [3, 2, 3]);
        assert_eq!(schedule.admit_ids::<Vec<_>>(&overlap, 0, true), [4, 5, 4]);
    }
}
