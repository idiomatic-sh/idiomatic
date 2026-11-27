//! The select front's driver-entry lists and the complete view lists of a file, kept by key for the file's visits.
use rustc_hash::FxHashMap;
use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use super::cold_bucket::Slot;
pub type Map = FxHashMap<usize, Slot>;
pub const SHIFT: usize = 56;
// Namespace tags of the two positive filtered driver-slot keys in select_kernel.
pub const KIND_ENTRY: usize = 3;
pub const LITERAL_ENTRY: usize = 4;
#[derive(Default)]
pub struct Rows {
    pub data: Map,
    pub filtering_entry: Cell<bool>,
}
impl Deref for Rows {
    type Target = Map;
    fn deref(&self) -> &Map {
        &self.data
    }
}
impl DerefMut for Rows {
    fn deref_mut(&mut self) -> &mut Map {
        &mut self.data
    }
}
impl Rows {
    #[inline]
    pub fn take_filtering_entry(&self) -> bool {
        if self.filtering_entry.get() {
            self.filtering_entry.set(false);
            true
        } else {
            false
        }
    }
    #[inline]
    pub fn get(&self, key: &usize) -> Option<&Rc<Vec<u32>>> {
        self.slot(key).map(Slot::owned)
    }
    pub fn slot(&self, key: &usize) -> Option<&Slot> {
        let rows = self.data.get(key);
        // Only the two cached driver-entry constructors next call view_cols::retain.
        // Transient per-key join hits use that function too, outside this scope.
        if matches!(*key >> SHIFT, KIND_ENTRY | LITERAL_ENTRY) && rows.is_none() {
            self.filtering_entry.set(true);
        }

        rows
    }
    #[inline]
    pub fn insert(&mut self, key: usize, rows: impl Into<Slot>) -> Option<Slot> {
        self.data.insert(key, rows.into())
    }
}
