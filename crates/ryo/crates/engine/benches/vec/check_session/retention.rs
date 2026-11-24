//! Only retain derived file state after a second demand. The immutable source snapshot
//! and previously returned answers have separate lifetimes.
use super::HeldFile;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Default)]
pub(crate) struct Retention<F: Default, D: Default> {
    seen: HashSet<usize>,
    files: HashMap<usize, (u64, u64, HeldFile<F, D>)>,
    oldest: BTreeMap<u64, usize>,
    sequence: u64,
    hits: u64,
    evictions: u64,
    high_water: u64,
    used: u64,
    limit: Option<u64>,
    allocated: Option<fn() -> Option<u64>>,
}

impl<F: Default, D: Default> Retention<F, D> {
    pub(crate) fn counts(&self) -> [u64; 4] {
        [
            self.seen.len() as u64,
            self.hits,
            self.evictions,
            self.high_water,
        ]
    }

    pub(crate) fn configure(&mut self, limit: Option<u64>, allocated: fn() -> Option<u64>) {
        // An entry admitted without a meter has no byte charge.
        if self.limit.is_none() && limit.is_some() {
            self.files.clear();
            self.oldest.clear();
            self.used = 0;
        }
        self.limit = limit;
        self.allocated = Some(allocated);
        self.trim();
    }

    pub(super) fn clock(&self) -> Option<u64> {
        if self.limit.is_none() {
            return Some(0);
        }
        self.allocated.and_then(|read| read())
    }

    pub(super) fn take(&mut self, index: usize) -> Option<(u64, HeldFile<F, D>)> {
        if self.seen.insert(index) {
            return None;
        }
        if let Some((stamp, bytes, file)) = self.files.remove(&index) {
            self.hits += 1;
            self.oldest.remove(&stamp);
            self.used -= bytes;
            return Some((bytes, file));
        }
        Some((0, HeldFile::default()))
    }

    pub(super) fn put(&mut self, index: usize, bytes: u64, file: HeldFile<F, D>) {
        if self.limit.is_some_and(|max| bytes > max) {
            self.evictions += 1;
            return;
        }
        self.sequence += 1;
        self.oldest.insert(self.sequence, index);
        self.used += bytes;
        self.files.insert(index, (self.sequence, bytes, file));
        self.trim();
        self.high_water = self.high_water.max(self.used);
    }

    fn trim(&mut self) {
        while self.limit.is_some_and(|limit| self.used > limit) {
            let (_, index) = self.oldest.pop_first().expect("charged file has an age");
            let (_, bytes, _) = self
                .files
                .remove(&index)
                .expect("age identifies a cached version");
            self.evictions += 1;
            self.used -= bytes;
        }
    }
}
