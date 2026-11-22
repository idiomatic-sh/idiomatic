//! THROWAWAY (ryo-fmx3.1, redo-empty): emptiness for the FILE, decided by propagation over the plan.
//!
//! Cycle 1 (`VEC_EP_COUNT=1`): the count. Every bracketed run that returns nothing (whole or by key; not the runs known
//! empty before the bracket) is recorded in the file; at the file's end each is classed by the cheapest leaf facts that,
//! propagated through the plan (a select needs every source, a union one input, the rest their first input: `known`'s
//! needs), make its step certainly empty in the file:
//! - 0 static: `known`'s leaves (kinds absent, an empty survivor set, a per-file test);
//! - 1 no rows: a provider relation whose fill has no rows in the file;
//! - 2 literal: a literal test no row of the file satisfies, from the values present in a small closed column (a fact's
//!   symbol column, a child's field, an element's text word);
//! - 3 view: a view (one relation and its tests) with no row in the file (its tests read, once a file);
//! - 4 data: none of these: only the join (or a probe's keys) can tell.

use super::{E, Exec, File, Native, Run, kinds_required, lit_rows, step_clock, view_rows};
use rustc_hash::FxHashSet;
use std::cell::RefCell;

pub(super) const CLASSES: [&str; 7] = ["static", "no-rows", "literal-fact", "literal-field", "literal-word", "view", "data"];
const DATA: usize = 6;
const VIEW: usize = 5;

#[derive(Default)]
struct Count {
    /// this file's empty runs: (step, by key, own seconds)
    runs: Vec<(u32, bool, f64)>,
    /// per class: whole runs, probes, seconds
    by: [(u64, u64, f64); 7],
    /// per (class, op): runs, seconds
    by_op: std::collections::BTreeMap<(usize, String), (u64, f64)>,
    /// step runs (whole + by key) in files under 2 KB, and those files
    small_runs: u64,
    small_files: u64,
    runs_total: u64,
    files: u64,
    /// level memo of the file: 0 unknown, else 1 + the class (5: never)
    lvl: Vec<u8>,
    words: Option<FxHashSet<u32>>,
    fields: Option<FxHashSet<u32>>,
    time: f64,
    /// leaf decisions 'no rows', by relation (a node a file)
    no_rows: std::collections::BTreeMap<String, u64>,
}

thread_local! {
    static C: RefCell<Count> = RefCell::new(Count::default());
    static RUNS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// A bracketed run (whole or by key) of a step.
#[inline]
pub(super) fn ran() {
    RUNS.with(|c| c.set(c.get() + 1));
}

/// A bracketed run that returned nothing (count mode only).
#[inline]
pub(super) fn empty_run(node: usize, probe: bool, own: f64) {
    if sw!("VEC_EP_COUNT") {
        C.with(|c| c.borrow_mut().runs.push((node as u32, probe, own)));
    }
}

/// The file's end: its runs counted (small files apart); in count mode its empty runs classed.
pub(crate) fn file_end(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run) {
    let n = RUNS.with(|c| c.replace(0));
    let small = f.src.len() < 2048;
    C.with(|c| {
        let mut c = c.borrow_mut();
        c.runs_total += n;
        c.files += 1;
        if small {
            c.small_runs += n;
            c.small_files += 1;
        }
    });
    if !sw!("VEC_EP_COUNT") {
        return;
    }
    let t0 = step_clock();
    let runs = C.with(|c| std::mem::take(&mut c.borrow_mut().runs));
    C.with(|c| {
        let mut c = c.borrow_mut();
        c.lvl.clear();
        c.lvl.resize(ex.plan.nodes.len(), 0);
        c.words = None;
        c.fields = None;
    });
    for &(node, probe, own) in &runs {
        let k = ex.ep_level(f, run, node as usize).min(DATA);
        C.with(|c| {
            let mut c = c.borrow_mut();
            let b = &mut c.by[k];
            if probe { b.1 += 1 } else { b.0 += 1 }
            b.2 += own;
            let e = c.by_op.entry((k, ex.plan.nodes[node as usize].op.clone())).or_default();
            e.0 += 1;
            e.1 += own;
        });
    }
    C.with(|c| c.borrow_mut().time += step_clock() - t0);
}

/// The report's lines.
pub(crate) fn print() {
    C.with(|c| {
        let c = c.borrow();
        println!("EPRUNS\tfiles {}\truns {}\tsmall files (<2 KB) {}\truns in them {}\ta small file {:.0}", c.files, c.runs_total, c.small_files, c.small_runs, if c.small_files > 0 { c.small_runs as f64 / c.small_files as f64 } else { 0.0 });
        if !sw!("VEC_EP_COUNT") {
            return;
        }
        println!("EPCOUNT\tclass\twhole\tprobes\tseconds (own, step clock)");
        for (i, (w, p, s)) in c.by.iter().enumerate() {
            println!("EPCOUNT\t{}\t{w}\t{p}\t{s:.3}", CLASSES[i]);
        }
        for ((k, op), (n, s)) in &c.by_op {
            println!("EPCLASSOP\t{}\t{op}\t{n}\t{s:.3}", CLASSES[*k]);
        }
        println!("EPCOUNT\tclassing seconds {:.3}", c.time);
        for (r, n) in &c.no_rows {
            println!("EPNOROWS\t{r}\t{n}");
        }
    });
}

impl Exec<'_> {
    /// The cheapest class of leaf facts that leaves `node` certainly empty in this file (4: none; 5: never knowable).
    fn ep_level(&mut self, f: &mut File<'_>, run: &mut Run, node: usize) -> usize {
        let m = C.with(|c| c.borrow().lvl[node]);
        if m != 0 {
            return usize::from(m - 1);
        }
        // a cycle is impossible; mark first all the same
        C.with(|c| c.borrow_mut().lvl[node] = DATA as u8 + 2);
        let l = self.ep_level_(f, run, node);
        C.with(|c| c.borrow_mut().lvl[node] = l as u8 + 1);
        l
    }

    fn ep_level_(&mut self, f: &mut File<'_>, run: &mut Run, node: usize) -> usize {
        if self.static_empty(f, run, node) {
            return 0;
        }
        let need = self.need(run, node);
        if need.never {
            return DATA;
        }
        let mut best = DATA;
        // the step's own leaf
        if let Ok(Some(v)) = self.view(run, node) {
            best = best.min(self.ep_leaf(f, run, v.base, &v.filter, 0, &[]));
            if best > VIEW && self.ep_fact_ready(f, v.base) && view_rows(f, run, &v, node).is_empty() {
                best = VIEW;
            }
            return best;
        }
        if self.plan.nodes[node].op == "select"
            && let Ok(sp) = self.sel(run, node)
        {
            for (i, s) in sp.srcs.iter().enumerate() {
                if let Ok(Some(v)) = self.view(run, *s) {
                    best = best.min(self.ep_leaf(f, run, v.base, &sp.conj, i, &v.map));
                }
            }
        }
        // through the inputs
        let ins: Vec<usize> = need.inputs.to_vec();
        if !ins.is_empty() {
            let ls: Vec<usize> = ins.iter().map(|i| self.ep_level(f, run, *i)).collect();
            let through = if need.all { ls.iter().copied().max().unwrap_or(DATA) } else { ls.iter().copied().min().unwrap_or(DATA) };
            best = best.min(through);
        }
        best
    }

    fn ep_fact_ready(&self, f: &File<'_>, base: Native) -> bool {
        match base {
            Native::Fact(i) => f.facts.get(usize::from(i)).is_some_and(Option::is_some),
            _ => true,
        }
    }

    /// A relation's leaf facts under a conjunction over its source `si` (`map`: the source's columns over the base's).
    fn ep_leaf(&mut self, f: &mut File<'_>, run: &mut Run, base: Native, conj: &[E], si: usize, map: &[usize]) -> usize {
        if let Native::Fact(i) = base {
            let rel = &run.fact_rels[usize::from(i)].0;
            let filled = f.facts.get(usize::from(i)).and_then(Option::as_ref).or_else(|| f.fills.get(rel));
            if filled.is_some_and(|t| t.n == 0) {
                if sw!("VEC_EP_COUNT") {
                    C.with(|c| *c.borrow_mut().no_rows.entry(rel.clone()).or_default() += 1);
                }
                return 1;
            }
            if self.ep_fact_ready(f, base) && lit_rows(f, conj, si, base, map).is_some_and(|r| r.is_empty()) {
                return 2;
            }
            return DATA;
        }
        let bc = |c: usize| if map.is_empty() { c } else { map[c] };
        let ncols = if map.is_empty() { 9 } else { map.len() };
        for e in conj {
            for c in 0..ncols {
                let b = bc(c);
                let want = match (base, b) {
                    (Native::Tree, 2) | (Native::Child, 3) => b,
                    _ => continue,
                };
                let Some(vals) = kinds_required(e, si, c) else { continue };
                let present = if want == 2 { self.ep_words(f, run) } else { self.ep_fields(f, run) };
                if vals.iter().all(|v| !present.contains(v)) {
                    return if want == 2 { 4 } else { 3 };
                }
            }
        }
        DATA
    }

    fn ep_words(&mut self, f: &File<'_>, run: &mut Run) -> FxHashSet<u32> {
        if let Some(w) = C.with(|c| c.borrow().words.clone()) {
            return w;
        }
        let w: FxHashSet<u32> = (0..f.n as u32).map(|p| f.elem_text(run, p)).collect();
        C.with(|c| c.borrow_mut().words = Some(w.clone()));
        w
    }

    fn ep_fields(&mut self, f: &File<'_>, run: &Run) -> FxHashSet<u32> {
        if let Some(w) = C.with(|c| c.borrow().fields.clone()) {
            return w;
        }
        let w: FxHashSet<u32> = (0..run.field_sym.len()).filter(|c| !super::node_table::ids_of_field(f.table, ryo::languages::FieldCode(*c as u16)).is_empty()).map(|c| run.field_sym[c]).collect();
        C.with(|c| c.borrow_mut().fields = Some(w.clone()));
        w
    }
}
