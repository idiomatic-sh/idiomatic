//! redo-fills (THROWAWAY, bead ryo-fmx3.1): the fills' and the native reads' own counters, every one behind a switch.
//! `VEC_FILL_TIME`: the fills' section clock (thread CPU when `VEC_THREAD_CLOCK`); off, no clock is read in the fill path.
//! `VEC_READ_COUNT`: calls of the native readers. `VEC_FILL_COUNT`: zero-row fills and the asking step kind. `VEC_FILL_DUMP=<file>`: per fill a line (file, relation, rows, a digest of
//! its rows with symbol cells as their texts), for the equality of fills across builds.

use super::{Run, Table, Ty};
use std::cell::{Cell, RefCell};
use std::hash::Hasher;

thread_local! {
    /// The step kind that asked the provider for the fill in flight.
    pub static SITE: Cell<&'static str> = const { Cell::new("other") };
    /// Calls: native_get, view_holds, lit_rows, view_rows, native_probe, native_rows; cells an expression read of a view, of a base.
    pub static READS: RefCell<[u64; 8]> = const { RefCell::new([0; 8]) };
    /// Per fact slot: files filled, files whose fill no step read (size only, or never).
    static FILLED: RefCell<Vec<(u64, u64)>> = const { RefCell::new(Vec::new()) };
    /// native_get calls by (relation kind: tree, child, location, token, fact; column).
    static AT: RefCell<[[u64; 16]; 5]> = const { RefCell::new([[0; 16]; 5]) };
    /// Per fact slot, the step that asked this file's fill (the top of the step stack); and the unread fills by
    /// (slot, asking step).
    static ASKER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static UNREAD_BY: RefCell<std::collections::HashMap<(usize, u32), u64>> = RefCell::new(std::collections::HashMap::new());
    static DUMP: RefCell<Option<std::io::BufWriter<std::fs::File>>> = const { RefCell::new(None) };
}


/// Whether the fills are counted by kind (zero-row fills, the step kind that asked): `VEC_FILL_COUNT`.
pub fn counting() -> bool {
    sw!("VEC_FILL_COUNT")
}

/// Counts a native read of reader `i` (under `VEC_READ_COUNT`).
#[inline]
pub fn read(i: usize) {
    if sw!("VEC_READ_COUNT") {
        READS.with(|r| r.borrow_mut()[i] += 1);
    }
}

/// Counts a native_get of a relation kind and column (under `VEC_READ_COUNT`).
#[inline]
pub fn read_at(kind: usize, col: usize) {
    if sw!("VEC_READ_COUNT") {
        AT.with(|r| r.borrow_mut()[kind][col.min(15)] += 1);
    }
}

/// Sets the step kind asking now; the previous comes back for the caller to restore.
#[inline]
pub fn site(s: &'static str) -> &'static str {
    SITE.with(|c| c.replace(s))
}




/// Whether `node` asks a fill for the first time in the run (for `VEC_ASKER_PLAN`'s one line a step).
pub fn first_asker(node: u32) -> bool {
    thread_local! { static SEEN: RefCell<std::collections::HashSet<u32>> = RefCell::new(std::collections::HashSet::new()); }
    SEEN.with(|s| s.borrow_mut().insert(node))
}

/// The step `node` asks fact slot `i`'s fill (under `VEC_FILL_COUNT`).
pub fn asked(i: usize, node: u32) {
    ASKER.with(|a| {
        let mut a = a.borrow_mut();
        if a.len() <= i {
            a.resize(i + 1, u32::MAX);
        }
        a[i] = node;
    });
}

/// A file's fill of fact slot `i` ended; `read`: a step read its rows.
pub fn fact_filled(i: usize, read: bool) {
    if !read {
        let by = ASKER.with(|a| a.borrow().get(i).copied().unwrap_or(u32::MAX));
        UNREAD_BY.with(|u| *u.borrow_mut().entry((i, by)).or_default() += 1);
    }
    FILLED.with(|f| {
        let mut f = f.borrow_mut();
        if f.len() <= i {
            f.resize(i + 1, (0, 0));
        }
        f[i].0 += 1;
        f[i].1 += u64::from(!read);
    });
}

