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

use super::{E, Exec, File, Native, Provider, Run, kinds_required, lit_rows, step_clock, view_rows};
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
pub(crate) fn file_end(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider) {
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
        let k = ex.ep_level(f, run, p, node as usize).min(DATA);
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
    fn ep_level(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> usize {
        let m = C.with(|c| c.borrow().lvl[node]);
        if m != 0 {
            return usize::from(m - 1);
        }
        // a cycle is impossible; mark first all the same
        C.with(|c| c.borrow_mut().lvl[node] = DATA as u8 + 2);
        let l = self.ep_level_(f, run, p, node);
        C.with(|c| c.borrow_mut().lvl[node] = l as u8 + 1);
        l
    }

    fn ep_level_(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> usize {
        if self.static_empty(f, run, p, node) {
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
            let ls: Vec<usize> = ins.iter().map(|i| self.ep_level(f, run, p, *i)).collect();
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

// The normal executor binds the propagation leaves by catalog type. Multiple steps
// may refer to the same premise in the file (static_tests.rs).

/// cycle 4: a view's rows are read for its emptiness only where its entry has at most this many rows.
const VIEW_ENTRY_CAP: usize = 64;


#[derive(Default)]
struct Leaves {
    /// runs not made because the propagation's leaves decided them, by step
    skipped: Vec<u64>,
    /// emits asked in a file: decided empty by the old leaves, by the propagation's, or run
    emits: [u64; 3],
}

thread_local! {
    static L: RefCell<Leaves> = RefCell::new(Leaves::default());
}

/// The columns of source `si` an expression names.
fn cols_of(e: &E, si: usize, out: &mut Vec<usize>) {
    match e {
        E::Col(s, c, _) if *s == si => out.push(*c),
        E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) | E::In(a, b, _) => {
            cols_of(a, si, out);
            cols_of(b, si, out);
        }
        _ => {}
    }
}

/// Resolve the same first literal column as lit_pick, from its catalog type.
fn bind_source(tests: &super::static_tests::Tests, out: &mut Vec<super::static_tests::Test>, run: &mut Run, node: usize,
               conj: &[E], si: usize, base: Native, map: &[usize], own: bool) {
    use super::static_tests::Test::*;
    let fact = if let Native::Fact(id) = base { Some(id) } else { None };
    let tys = fact.and_then(|id| {
        let name = run.fact_rels[usize::from(id)].0.clone();
        run.catalog(&name).map(|(_, _, tys)| tys)
    });
    let mut has_literal = false;
    let mut literal = None;
    let mut fields = Vec::new();
    let mut columns = Vec::new();
    for expr in conj {
        columns.clear();
        cols_of(expr, si, &mut columns);
        for &c in &columns {
            let Some(mut values) = kinds_required(expr, si, c) else { continue };
            let bc = if map.is_empty() { c } else { map[c] };
            values.sort_unstable();
            values.dedup();
            if fact.is_some() {
                has_literal = true;
                if literal.is_none() && tys.as_ref().and_then(|ts| ts.get(bc)) == Some(&super::Ty::Sym) {
                    literal = Some((bc, tests.words(run, &values)));
                }
            } else if base == Native::Child && bc == 3 && fields.is_empty() {
                fields = values;
            }
        }
    }
    if !fields.is_empty() { out.push(Fields(tests.words(run, &fields))); }
    if own && sw!("VEC_EP_VIEW") && (has_literal || super::kind_set(run, conj, si, base, map).is_some()) {
        out.push(SmallView(node));
    }
    if let Some(id) = fact {
        if own { out.push(EmptyFact(id)); }
        if let Some((col, values)) = literal { out.push(Literal(id, col, values)); }
        else if !own && has_literal { out.push(DemandFact(id)); }
    }
}

/// An emit asked in a file: answered before running anything (its step decided empty by the file's leaves: the old
/// ones, or the propagation's), or run.
pub(crate) fn emit_done(f: &File<'_>, node: usize) {
    let k = match f.known[node] {
        super::known::YES_EP => 1,
        2 => 0,
        _ => 2,
    };
    L.with(|l| l.borrow_mut().emits[k] += 1);
}

/// A step skipped (answered empty before the bracket) because the propagation's leaves decided it.
#[inline]
pub(super) fn skipped(f: &File<'_>, node: usize) {
    if f.known[node] == super::known::YES_EP {
        L.with(|l| {
            let mut l = l.borrow_mut();
            if l.skipped.len() <= node {
                l.skipped.resize(node + 1, 0);
            }
            l.skipped[node] += 1;
        });
    }
}

/// The skipped runs by step kind (report lines `EPSKIP`).
pub(crate) fn print_skipped(ops: &dyn Fn(usize) -> String) {
    L.with(|l| {
        let l = l.borrow();
        let mut by: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
        for (n, k) in l.skipped.iter().enumerate() {
            if *k > 0 {
                *by.entry(ops(n)).or_default() += k;
            }
        }
        let total: u64 = by.values().sum();
        println!("EPSKIP\tall\t{total}");
        println!("EPEMIT\temits asked in a file {}: decided empty before running by the old leaves {}, by the propagation's {}; run {}", l.emits.iter().sum::<u64>(), l.emits[0], l.emits[1], l.emits[2]);
        for (op, k) in by {
            println!("EPSKIP\t{op}\t{k}");
        }
    });
}

impl Exec<'_> {
    pub(super) fn bind_ep(&mut self, run: &mut Run, node: usize) -> super::static_tests::List {
        let mut out = Vec::new();
        if let Ok(Some(v)) = self.view(run, node) {
            bind_source(&self.tests, &mut out, run, node, &v.filter, 0, v.base, &[], true);
        } else if self.plan.nodes[node].op == "select" && let Ok(sp) = self.sel(run, node) {
            for (i, &source) in sp.srcs.iter().enumerate() {
                if let Ok(Some(v)) = self.view(run, source) {
                    bind_source(&self.tests, &mut out, run, node, &sp.conj, i, v.base, &v.map, false);
                }
            }
        }
        self.tests.bind(out)
    }

    /// cycle 4: whether a view has no row in this file (its entry, then its tests), read only where its entry is small.
    pub(super) fn ep_view_rows_empty(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> bool {
        let Ok(Some(v)) = self.view(run, node) else { return false };
        if self.load_fact(f, run, p, v.base).is_err() || !self.ep_fact_ready(f, v.base) {
            return false;
        }
        let entry = match super::kind_set(run, &v.filter, 0, v.base, &[]) {
            Some(ks) => super::bucket_len(f, run, &ks),
            None => match lit_rows(f, &v.filter, 0, v.base, &[]) {
                Some(r) => r.len(),
                None => return false,
            },
        };
        entry <= VIEW_ENTRY_CAP && view_rows(f, run, &v, node).is_empty()
    }

    /// Whether the file has a child under one of these field names.
    pub(super) fn ep_field_present(&self, f: &File<'_>, run: &Run, names: &[super::static_tests::Word]) -> bool {
        let fields = f.present_fields.get_or_init(|| run.field_sym.iter().enumerate()
            .filter(|(code, _)| !super::node_table::ids_of_field(f.table, ryo::languages::FieldCode(*code as u16)).is_empty())
            .map(|(_, &name)| name).collect());
        names.iter().filter_map(|word| word.id(run)).any(|name| fields.contains(&name))
    }
}
