//! The `keyed_ranges` service: ask, file and gather a probed step's rows by key, one lookup a key.

use super::constants::{DENSE_MIN_KEYS, DENSE_SHARE, PENDING};
use super::types::{Asked, AskedRange, KeyRange, KeyedRanges, RangeSlot, SlotId};

/// A store's new slot, of `width` columns.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::keyed_ranges::service::{new_slot, width};
/// use ryo_engine::domain::keyed_ranges::types::KeyedRanges;
/// let mut store = KeyedRanges::default();
/// let slot = new_slot(&mut store, 3);
/// assert_eq!(width(&store, slot), 3);
/// ```
pub fn new_slot(store: &mut KeyedRanges, width: usize) -> SlotId {
    let id = SlotId(u32::try_from(store.slots.len()).unwrap_or(u32::MAX));
    store.slots.push(RangeSlot { columns: vec![Vec::new(); width], ..RangeSlot::default() });
    id
}

/// How many columns a slot's rows have.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::keyed_ranges::service::{new_slot, width};
/// use ryo_engine::domain::keyed_ranges::types::KeyedRanges;
/// let mut store = KeyedRanges::default();
/// let slot = new_slot(&mut store, 2);
/// assert_eq!(width(&store, slot), 2);
/// ```
#[must_use]
pub fn width(store: &KeyedRanges, slot: SlotId) -> usize {
    store.slots[slot.0 as usize].columns.len()
}

/// Starts the next file: every slot used in the last is emptied by resetting its lengths (its buffers kept); keys below
/// `dense_bound` (the file's row or node count) are found by position from now on.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::keyed_ranges::service::{ask, file, new_slot, next_file};
/// use ryo_engine::domain::keyed_ranges::types::{Asked, KeyedRanges};
/// let mut store = KeyedRanges::default();
/// next_file(&mut store, 10);
/// let slot = new_slot(&mut store, 1);
/// let (mut missing, mut asked) = (Vec::new(), Vec::new());
/// let _ = ask(&mut store, slot, &[4], &mut missing, &mut asked);
/// let _ = file(&mut store, slot, &missing, 0, &[&[4]], 1);
/// next_file(&mut store, 10);
/// missing.clear();
/// assert_eq!(ask(&mut store, slot, &[4], &mut missing, &mut asked), Asked::Missing);
/// ```
pub fn next_file(store: &mut KeyedRanges, dense_bound: u64) {
    let last = store.dense_bound;
    for slot in std::mem::take(&mut store.touched) {
        let s = &mut store.slots[slot.0 as usize];
        s.touched = false;
        s.columns.iter_mut().for_each(Vec::clear);
        s.sparse.clear();
        // a slot that held many keys keeps its position array (its entries die by generation, never by clearing)
        s.dense_mode = s.dense_mode || (s.keys >= DENSE_MIN_KEYS && u64::from(s.keys) * DENSE_SHARE >= last);
        s.keys = 0;
    }
    store.generation = store.generation.wrapping_add(1).max(1);
    store.last_bound = last;
    store.dense_bound = dense_bound;
}

fn entry(slot: &mut RangeSlot, dense_bound: u64, generation: u32, key: u64) -> Option<&mut KeyRange> {
    if slot.dense_mode && key < dense_bound {
        let at = usize::try_from(key).unwrap_or(usize::MAX);
        if at >= slot.dense.len() {
            return None;
        }
        let e = &mut slot.dense[at];
        return (e.generation == generation).then_some(e);
    }
    slot.sparse.get_mut(&key)
}

fn insert(slot: &mut RangeSlot, dense_bound: u64, generation: u32, key: u64, start: u32) -> &mut KeyRange {
    let fresh = KeyRange { start, length: 0, generation, stamp: 0 };
    slot.keys = slot.keys.saturating_add(1);
    if slot.dense_mode && key < dense_bound {
        let at = usize::try_from(key).unwrap_or(usize::MAX);
        if at >= slot.dense.len() {
            let to = usize::try_from(dense_bound).unwrap_or(at + 1).max(at + 1);
            slot.dense.resize(to, KeyRange::default());
        }
        slot.dense[at] = fresh;
        return &mut slot.dense[at];
    }
    let e = slot.sparse.entry(key).or_insert(fresh);
    *e = fresh;
    e
}

fn touch(store: &mut KeyedRanges, slot: SlotId) {
    let s = &mut store.slots[slot.0 as usize];
    if !s.touched {
        s.touched = true;
        store.touched.push(slot);
    }
}

/// Asks a slot for `keys`, one lookup a key: the keys not filed in this file are appended to `missing` (each once) and
/// marked pending; `asked` gets every key once, in list order, as `gather` will read it.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::keyed_ranges::service::{ask, new_slot, next_file};
/// use ryo_engine::domain::keyed_ranges::types::{Asked, KeyedRanges};
/// let mut store = KeyedRanges::default();
/// next_file(&mut store, 100);
/// let slot = new_slot(&mut store, 1);
/// let (mut missing, mut asked) = (Vec::new(), Vec::new());
/// assert_eq!(ask(&mut store, slot, &[7, 7, 1000], &mut missing, &mut asked), Asked::Missing);
/// assert_eq!(missing, vec![7, 1000]);
/// ```
pub fn ask(store: &mut KeyedRanges, slot: SlotId, keys: &[u64], missing: &mut Vec<u64>, asked: &mut Vec<AskedRange>) -> Asked {
    touch(store, slot);
    store.stamp = store.stamp.wrapping_add(1);
    let (bound, generation, stamp) = (store.dense_bound, store.generation, store.stamp);
    let s = &mut store.slots[slot.0 as usize];
    let before = missing.len();
    asked.clear();
    for (at, &k) in keys.iter().enumerate() {
        match entry(s, bound, generation, k) {
            Some(e) => {
                if e.stamp != stamp {
                    e.stamp = stamp;
                    if e.start != PENDING && e.length > 0 {
                        asked.push(AskedRange { start: e.start, length: e.length });
                    }
                }
            }
            None => {
                insert(s, bound, generation, k, PENDING).stamp = stamp;
                missing.push(k);
                asked.push(AskedRange { start: PENDING, length: u32::try_from(at).unwrap_or(u32::MAX) });
            }
        }
    }
    if missing.len() == before { Asked::Filed } else { Asked::Missing }
}

/// Forgets keys asked whose step failed: they are absent again.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::keyed_ranges::service::{ask, forget, new_slot, next_file};
/// use ryo_engine::domain::keyed_ranges::types::{Asked, KeyedRanges};
/// let mut store = KeyedRanges::default();
/// next_file(&mut store, 100);
/// let slot = new_slot(&mut store, 1);
/// let (mut missing, mut asked) = (Vec::new(), Vec::new());
/// let _ = ask(&mut store, slot, &[3], &mut missing, &mut asked);
/// forget(&mut store, slot, &missing);
/// assert_eq!(ask(&mut store, slot, &[3], &mut Vec::new(), &mut asked), Asked::Missing);
/// ```
pub fn forget(store: &mut KeyedRanges, slot: SlotId, keys: &[u64]) {
    let bound = store.dense_bound;
    let s = &mut store.slots[slot.0 as usize];
    for &k in keys {
        if s.dense_mode && k < bound {
            if let Some(e) = s.dense.get_mut(usize::try_from(k).unwrap_or(usize::MAX)) {
                *e = KeyRange::default();
            }
        } else {
            let _ = s.sparse.remove(&k);
        }
    }
}

/// Files a batch's rows (`batch`, one slice a column, `rows` long) for the keys asked (`asked`, the missing list): each
/// key's rows become one range, in batch order within a key. A key asked with no rows is filed empty; rows of a key not
/// asked are filed too (a step may answer more than it was asked). Answers whether a key filed before got more rows (its
/// range moved: an answer made from the old range is stale).
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::keyed_ranges::service::{ask, file, gather, new_slot, next_file};
/// use ryo_engine::domain::keyed_ranges::types::KeyedRanges;
/// let mut store = KeyedRanges::default();
/// next_file(&mut store, 100);
/// let slot = new_slot(&mut store, 2);
/// let (mut missing, mut asked) = (Vec::new(), Vec::new());
/// let _ = ask(&mut store, slot, &[6, 5], &mut missing, &mut asked);
/// assert!(!file(&mut store, slot, &missing, 0, &[&[6, 5, 6], &[1, 2, 3]], 3));
/// let mut out = vec![Vec::new(), Vec::new()];
/// assert_eq!(gather(&mut store, slot, &[6, 5], &mut asked, false, &mut out), 3);
/// assert_eq!(out, vec![vec![6, 6, 5], vec![1, 3, 2]]);
/// ```
pub fn file(store: &mut KeyedRanges, slot: SlotId, asked: &[u64], key_column: usize, batch: &[&[u64]], rows: usize) -> bool {
    let mut moved = false;
    touch(store, slot);
    let (bound, generation) = (store.dense_bound, store.generation);
    let mut order = std::mem::take(&mut store.order);
    order.clear();
    order.extend((0..rows).map(|r| (batch[key_column][r], u32::try_from(r).unwrap_or(u32::MAX))));
    order.sort_unstable();
    let s = &mut store.slots[slot.0 as usize];
    for &k in asked {
        if let Some(e) = entry(s, bound, generation, k) {
            e.start = 0;
            e.length = 0;
        }
    }
    let mut i = 0;
    while i < order.len() {
        let k = order[i].0;
        let mut j = i;
        while j < order.len() && order[j].0 == k {
            j += 1;
        }
        let start = u32::try_from(s.columns.first().map_or(0, Vec::len)).unwrap_or(u32::MAX);
        let (old_start, old_length) = match entry(s, bound, generation, k) {
            Some(e) if e.start != PENDING => (e.start as usize, e.length as usize),
            _ => (0, 0),
        };
        if old_length > 0 {
            moved = true;
            // rows of a key filed before: its old range moves to the end first, so the key's rows stay one range
            for c in &mut s.columns {
                c.extend_from_within(old_start..old_start + old_length);
            }
        }
        for (c, column) in s.columns.iter_mut().enumerate() {
            column.extend(order[i..j].iter().map(|x| batch[c][x.1 as usize]));
        }
        let length = u32::try_from(old_length + (j - i)).unwrap_or(u32::MAX);
        let e = insert(s, bound, generation, k, start);
        e.length = length;
        i = j;
    }
    store.order = order;
    moved
}

/// Appends the rows of an asked list to `out` (one buffer a column), in list order, each key once; answers how many rows.
/// `moved` (what `file` answered) makes every key be found again, since a range read by `ask` may have moved.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::keyed_ranges::service::{ask, file, gather, new_slot, next_file};
/// use ryo_engine::domain::keyed_ranges::types::KeyedRanges;
/// let mut store = KeyedRanges::default();
/// next_file(&mut store, 100);
/// let slot = new_slot(&mut store, 1);
/// let (mut missing, mut asked) = (Vec::new(), Vec::new());
/// let _ = ask(&mut store, slot, &[9, 9], &mut missing, &mut asked);
/// let moved = file(&mut store, slot, &missing, 0, &[&[9, 9]], 2);
/// let mut out = vec![Vec::new()];
/// assert_eq!(gather(&mut store, slot, &[9, 9], &mut asked, moved, &mut out), 2);
/// ```
pub fn gather(store: &mut KeyedRanges, slot: SlotId, keys: &[u64], asked: &mut Vec<AskedRange>, moved: bool, out: &mut [Vec<u64>]) -> usize {
    let (bound, generation) = (store.dense_bound, store.generation);
    if moved {
        let mut missing = Vec::new();
        let _ = ask(store, slot, keys, &mut missing, asked);
    }
    let s = &mut store.slots[slot.0 as usize];
    let mut total = 0usize;
    for r in asked.iter_mut() {
        if r.start == PENDING {
            let k = keys[r.length as usize];
            *r = match entry(s, bound, generation, k) {
                Some(e) if e.start != PENDING => AskedRange { start: e.start, length: e.length },
                _ => AskedRange::default(),
            };
        }
        total += r.length as usize;
    }
    for (c, column) in s.columns.iter().enumerate() {
        let o = &mut out[c];
        o.reserve(total);
        for r in asked.iter() {
            o.extend_from_slice(&column[r.start as usize..(r.start + r.length) as usize]);
        }
    }
    total
}
