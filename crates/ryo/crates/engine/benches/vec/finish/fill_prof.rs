//! redo-fills (THROWAWAY, bead ryo-fmx3.1): the fills' and the native reads' own counters, every one behind a switch.
//! `VEC_FILL_TIME`: the fills' section clock (thread CPU when `VEC_THREAD_CLOCK`); off, no clock is read in the fill path.
//! `VEC_READ_COUNT`: calls of the native readers. `VEC_FILL_COUNT`: filled and unread relation slots. `VEC_FILL_DUMP=<file>`: per fill a line (file, relation, rows, a digest of
//! its rows with symbol cells as their texts), for the equality of fills across builds.

use super::{Run, Table, Ty};
use std::cell::RefCell;
use std::hash::Hasher;

thread_local! {
    /// Calls: native_get, view_holds, lit_rows, view_rows, native_probe, native_rows; cells an expression read of a view, of a base.
    pub(in super::super) static READS: RefCell<[u64; 8]> = const { RefCell::new([0; 8]) };
    /// Per fact slot: files filled, files whose fill no step read (size only, or never).
    static FILLED: RefCell<Vec<(u64, u64)>> = const { RefCell::new(Vec::new()) };
    /// native_get calls by (relation kind: tree, child, location, token, fact; column).
    static AT: RefCell<[[u64; 16]; 5]> = const { RefCell::new([[0; 16]; 5]) };
    static DUMP: RefCell<Option<std::io::BufWriter<std::fs::File>>> = const { RefCell::new(None) };
}

/// Whether the fills' section clock runs.
pub(in super::super) fn timed() -> bool {
    sw!("VEC_FILL_TIME")
}

/// Whether the filled and unread slots are counted: `VEC_FILL_COUNT`.
pub(in super::super) fn counting() -> bool {
    sw!("VEC_FILL_COUNT")
}

/// Counts a native read of reader `i` (under `VEC_READ_COUNT`).
#[inline]
pub(in super::super) fn read(i: usize) {
    if sw!("VEC_READ_COUNT") {
        READS.with(|r| r.borrow_mut()[i] += 1);
    }
}

/// Counts a native_get of a relation kind and column (under `VEC_READ_COUNT`).
#[inline]
pub(in super::super) fn read_at(kind: usize, col: usize) {
    if sw!("VEC_READ_COUNT") {
        AT.with(|r| r.borrow_mut()[kind][col.min(15)] += 1);
    }
}

/// The reads line for the report.
pub(in super::super) fn reads_line() -> String {
    let r = READS.with(|r| *r.borrow());
    let at = AT.with(|a| *a.borrow());
    let by: Vec<String> = ["tree", "child", "location", "token", "fact"].iter().enumerate().flat_map(|(k, n)| at[k].iter().enumerate().filter(|(_, v)| **v > 0).map(move |(c, v)| format!("{n}.{c}={v}"))).collect();
    format!("native_get by relation.column: {}\nnative reads: native_get {} view_holds {} lit_rows {} view_rows {} native_probe {} native_rows {} (cells read by a step's expression: of a view {}, of a base {})", by.join(" "), r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7])
}

pub(in super::super) fn dumping() -> bool {
    sw!("VEC_FILL_DUMP")
}

/// One fill's line in the dump (under `VEC_FILL_DUMP`).
pub(in super::super) fn dump(run: &Run, file: u32, rel: &str, t: &Table) {
    static PATH: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    let Some(path) = PATH.get_or_init(|| std::env::var("VEC_FILL_DUMP").ok()) else { return };
    let mut h = rustc_hash::FxHasher::default();
    let mut rows: Vec<u64> = Vec::with_capacity(t.n);
    for r in 0..t.n {
        let mut rh = rustc_hash::FxHasher::default();
        for c in 0..t.cols.len() {
            let v = t.cols[c][r];
            if t.tys[c] == Ty::Sym && v != super::NULL {
                rh.write(&run.texts[v as usize]);
                rh.write_u8(0xff);
            } else {
                rh.write_u64(v);
            }
        }
        rows.push(rh.finish());
    }
    for v in &rows {
        h.write_u64(*v);
    }
    let ordered = h.finish();
    rows.sort_unstable();
    let mut hs = rustc_hash::FxHasher::default();
    for v in &rows {
        hs.write_u64(*v);
    }
    let line = format!("{}\t{rel}\t{}\t{}\t{:016x}\t{:016x}\n", run.paths[file as usize], t.n, t.names.join(","), hs.finish(), ordered);
    DUMP.with(|d| {
        let mut d = d.borrow_mut();
        let w = d.get_or_insert_with(|| std::io::BufWriter::new(std::fs::File::create(path).expect("fill dump")));
        std::io::Write::write_all(w, line.as_bytes()).expect("fill dump write");
    });
}

/// Flushes the dump.
pub(in super::super) fn flush() {
    DUMP.with(|d| {
        if let Some(w) = d.borrow_mut().as_mut() {
            std::io::Write::flush(w).expect("fill dump flush");
        }
    });
}

/// A file's fill of fact slot `i` ended; `read`: a step read its rows.
pub(in super::super) fn fact_filled(i: usize, read: bool) {
    FILLED.with(|f| {
        let mut f = f.borrow_mut();
        if f.len() <= i {
            f.resize(i + 1, (0, 0));
        }
        f[i].0 += 1;
        f[i].1 += u64::from(!read);
    });
}

/// Filled and unread relation slots, counted across files.
pub(in super::super) fn unread_lines(run: &Run) -> Vec<String> {
    FILLED.with(|f| f.borrow().iter().enumerate().filter(|(_, v)| v.0 > 0)
        .map(|(i, v)| format!("UNREAD\t{}\t{}\t{}", run.fact_rels[i].0, v.0, v.1)).collect())
}
