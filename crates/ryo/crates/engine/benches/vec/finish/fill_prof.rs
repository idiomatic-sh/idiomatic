//! redo-fills (THROWAWAY, bead ryo-fmx3.1): the fills' and the native reads' own counters, every one behind a switch.
//! `VEC_FILL_TIME`: the fills' section clock (thread CPU when `VEC_THREAD_CLOCK`); off, no clock is read in the fill path.
//! `VEC_READ_COUNT`: calls of the native readers. `VEC_FILL_COUNT`: zero-row fills and the asking step kind. `VEC_FILL_DUMP=<file>`: per fill a line (file, relation, rows, a digest of
//! its rows with symbol cells as their texts), for the equality of fills across builds.

use super::{Run, Table, Ty};
use std::cell::{Cell, RefCell};
use std::hash::Hasher;

thread_local! {
    /// The step kind that asked the provider for the fill in flight.
    pub(in super::super) static SITE: Cell<&'static str> = const { Cell::new("other") };
    /// Calls: native_get, view_holds, lit_rows, view_rows, native_probe, native_rows; cells an expression read of a view, of a base.
    pub(in super::super) static READS: RefCell<[u64; 8]> = const { RefCell::new([0; 8]) };
    /// Per fact slot: files filled, files whose fill no step read (size only, or never).
    static FILLED: RefCell<Vec<(u64, u64)>> = const { RefCell::new(Vec::new()) };
    /// native_get calls by (relation kind: tree, child, location, token, fact; column).
    static AT: RefCell<[[u64; 16]; 5]> = const { RefCell::new([[0; 16]; 5]) };
    /// Per fact slot, the step that asked this file's fill (the top of the step stack); and the unread fills by
    /// (slot, asking step).
    static ASKER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static UNREAD_BY: RefCell<std::collections::HashMap<(usize, u32), u64>> = RefCell::new(std::collections::HashMap::new());
}

/// Whether the fills' section clock runs.
pub(in super::super) fn timed() -> bool {
    sw!("VEC_FILL_TIME")
}

/// Whether the fills are counted by kind (zero-row fills, the step kind that asked): `VEC_FILL_COUNT`.
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

/// Sets the step kind asking now; the previous comes back for the caller to restore.
#[inline]
pub(in super::super) fn site(s: &'static str) -> &'static str {
    SITE.with(|c| c.replace(s))
}

/// The reads line for the report.
pub(in super::super) fn reads_line() -> String {
    let r = READS.with(|r| *r.borrow());
    let at = AT.with(|a| *a.borrow());
    let by: Vec<String> = ["tree", "child", "location", "token", "fact"].iter().enumerate().flat_map(|(k, n)| at[k].iter().enumerate().filter(|(_, v)| **v > 0).map(move |(c, v)| format!("{n}.{c}={v}"))).collect();
    format!("native_get by relation.column: {}\nnative reads: native_get {} view_holds {} lit_rows {} view_rows {} native_probe {} native_rows {} (cells read by a step's expression: of a view {}, of a base {})", by.join(" "), r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7])
}

/// One fill's line in the dump (under `VEC_FILL_DUMP`).
pub(in super::super) fn dump(run: &mut Run, file: u32, rel: &str, t: &Table) {
    static PATH: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    if PATH.get_or_init(|| std::env::var("VEC_FILL_DUMP").ok()).is_none() { return; }
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
    run.fill_dump.push((file, line));
}

/// Flush complete per-file traces after all worker shards have joined.
pub(in super::super) fn flush(run: &mut Run) {
    use std::io::Write;
    if run.fill_dump.is_empty() { return; }
    let path = std::env::var_os("VEC_FILL_DUMP").expect("configured fill dump");
    let mut output = std::io::BufWriter::new(std::fs::File::create(path).expect("fill dump"));
    run.fill_dump.sort_by_key(|(file, _)| *file);
    for (_, line) in run.fill_dump.drain(..) {
        output.write_all(line.as_bytes()).expect("fill dump write");
    }
    output.flush().expect("fill dump flush");
}

pub(crate) struct Counts {
    reads: [u64; 8],
    columns: [[u64; 16]; 5],
    relations: Vec<(String, usize)>,
    filled: Vec<(u64, u64)>,
    unread: std::collections::HashMap<(usize, u32), u64>,
}

impl Counts {
    pub(crate) fn take(run: &Run) -> Option<Box<Self>> {
        if !counting() && !sw!("VEC_READ_COUNT") { return None; }
        Some(Box::new(Self {
            reads: READS.take(),
            columns: AT.take(),
            relations: run.fact_rels.clone(),
            filled: FILLED.take(),
            unread: UNREAD_BY.take(),
        }))
    }

    pub(crate) fn merge(self, run: &mut Run) {
        READS.with_borrow_mut(|totals| {
            for (total, part) in totals.iter_mut().zip(self.reads) { *total += part; }
        });
        AT.with_borrow_mut(|totals| {
            for (total, part) in totals.iter_mut().flatten().zip(self.columns.into_iter().flatten()) { *total += part; }
        });
        // Fact slots are local to an executor. Compare and report their declared names.
        let slots: Vec<_> = self.relations.into_iter().map(|relation| {
            run.fact_rels.iter().position(|r| r.0 == relation.0).unwrap_or_else(|| {
                run.fact_rels.push(relation);
                run.fact_rels.len() - 1
            })
        }).collect();
        FILLED.with_borrow_mut(|totals| {
            totals.resize(totals.len().max(run.fact_rels.len()), (0, 0));
            for (i, (fills, unread)) in self.filled.into_iter().enumerate() {
                totals[slots[i]].0 += fills;
                totals[slots[i]].1 += unread;
            }
        });
        UNREAD_BY.with_borrow_mut(|totals| {
            for ((i, node), count) in self.unread {
                *totals.entry((slots[i], node)).or_default() += count;
            }
        });
    }
}

/// Whether `node` asks a fill for the first time in the run (for `VEC_ASKER_PLAN`'s one line a step).
pub(in super::super) fn first_asker(node: u32) -> bool {
    thread_local! { static SEEN: RefCell<std::collections::HashSet<u32>> = RefCell::new(std::collections::HashSet::new()); }
    SEEN.with(|s| s.borrow_mut().insert(node))
}

/// The step `node` asks fact slot `i`'s fill (under `VEC_FILL_COUNT`).
pub(in super::super) fn asked(i: usize, node: u32) {
    ASKER.with(|a| {
        let mut a = a.borrow_mut();
        if a.len() <= i {
            a.resize(i + 1, u32::MAX);
        }
        a[i] = node;
    });
}

/// A file's fill of fact slot `i` ended; `read`: a step read its rows.
pub(in super::super) fn fact_filled(i: usize, read: bool) {
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

/// The fills no step read, by relation, for the report; then by relation and asking step.
pub(in super::super) fn unread_lines(run: &Run) -> Vec<String> {
    let mut out: Vec<String> = FILLED.with(|f| f.borrow().iter().enumerate().filter(|(_, v)| v.0 > 0).map(|(i, v)| format!("UNREAD\t{}\t{}\t{}", run.fact_rels[i].0, v.0, v.1)).collect());
    UNREAD_BY.with(|u| out.extend(u.borrow().iter().map(|((i, n), c)| format!("UNREAD-BY\t{}\t{}\t{c}", run.fact_rels[*i].0, *n as i64))));
    out
}
