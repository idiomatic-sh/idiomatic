//! THROWAWAY (ryo-fmx3.1, redo-union pass 2): a step run known to return nothing before it is entered. What the file
//! already knows decides it, with no table, no stack push and no clock:
//! - its kind buckets: a view, or a select's tree source, whose test names kinds none of which the file has;
//! - its survivor sets: a select with a seed test on a set that is empty in the file;
//! - a per-file test that fails (the path tests `filter_order` decides);
//! - an input the step needs: any source of a select, the first input of every other step (the left of an antijoin,
//!   the from of a pass-through step, a closure's edges), every input of a union: known empty by the above, or made
//!   empty already in this file.
//!
//! Exactly the cases where the step's body returns nothing before reading anything else (`Exec::body`, `select`). The
//! static part (kinds, seeds, file tests, through the inputs) is kept a file; the made part is read live (a probe's
//! substitution changes what is made, never what the file holds).

use super::{E, Exec, File, Native, Provider, Res, Run, filter_order};

/// Inputs and local proof premises resolved while binding the step.
pub(super) struct Need {
    /// Empty when any of these is empty (or, `all`, when every one is).
    pub(super) inputs: Box<[usize]>,
    pub(super) all: bool,
    /// Kind sets the rows must have one of (kind codes): empty when the file has none of a set.
    pub(super) kinds: Box<[Box<[u16]>]>,
    /// Survivor sets a seed test reads.
    pub(super) seeds: Box<[usize]>,
    /// A per-file test to decide.
    path: filter_order::PathList,
    /// Never known empty (a constant step: made once a run, as before).
    pub(super) never: bool,
}

#[derive(Clone, Copy)]
pub(super) enum AskUse {
    Whole,
    Keyed,
    Size,
}

const NO: u8 = 1;
const YES: u8 = 2;
const BUSY: u8 = 3;

#[cfg(feature = "vec-ask-audit")]
pub(super) fn proved(value: u8) -> bool {
    matches!(value, YES | YES_EP)
}
/// redo-empty: known empty by the propagation's own leaves (directly or through an input): counted apart.
pub(super) const YES_EP: u8 = 4;

/// pass 2: a step's needs in flat arrays (one record a step, its lists in shared arrays), for the per-file decisions:
/// the inputs kept are those that can be known empty themselves (`CAN`); a step none of whose needs can is never
/// decided a file.
#[derive(Clone, Copy, Default)]
pub(super) struct Rec {
    ins: u32,
    n_ins: u16,
    first: super::static_tests::List,
    propagated: super::static_tests::List,
    path: filter_order::PathList,
    flags: u8,
}
const READY: u8 = 1;
const NEVER: u8 = 2;
const ALL: u8 = 4;
const FILE_TEST: u8 = 8;
const CAN: u8 = 16;
/// redo-empty: the step has a leaf of the propagation (`emptyprop`).
const EP_LEAF: u8 = 32;
const PUBLISH: u8 = 64;

#[derive(Default)]
pub(super) struct Flat {
    recs: Vec<Rec>,
    ins: Vec<u32>,
    publication: Vec<(u32,u32)>,
    publish_edges: Vec<u32>,
}

impl Flat {
    pub(super) fn new(nodes: usize) -> Self {
        Flat { recs: vec![Rec::default(); nodes], ..Flat::default() }
    }
}

impl Exec<'_> {
    /// Whether a run of `node` in this file returns nothing, known without running it.
    #[inline(always)]
    pub(super) fn known_empty(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, _usage: AskUse) -> bool {
        let empty = self.known_empty_now(f, run, p, node);
        #[cfg(feature = "vec-ask-audit")]
        super::ask_audit::observe(self, f, node, _usage, empty);
        empty
    }

    #[inline(always)]
    fn known_empty_now(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> bool {
        match f.known[node] {
            YES | YES_EP => return true,
            NO | BUSY => {}
            _ => {
                if self.static_empty_(f, run, p, node) {
                    return true;
                }
            }
        }
        let r = self.rec(run, node);
        if r.flags & NEVER != 0 {
            return false;
        }
        // made empty already (read live): any source of a select, every input of a union, the first input of the rest
        let made = |i: usize| matches!(&f.memo[i], Some(Res::Tab(t)) if t.n == 0);
        match self.frames[node].op {
            super::frame::Op::Select => self.plan.nodes[node].inputs.iter().any(|i| made(*i)),
            super::frame::Op::Union => {
                let ins = &self.plan.nodes[node].inputs;
                !ins.is_empty() && ins.iter().all(|i| made(*i))
            }
            _ => {
                let i = self.frames[node].first;
                i < super::frame::MISSING && made(i as usize)
            }
        }
    }

    /// Whether the file's kinds, survivor sets or per-file tests leave `node` empty (kept a file).
    #[inline]
    pub(super) fn static_empty(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> bool {
        match f.known[node] {
            NO | BUSY => false,
            YES | YES_EP => true,
            _ => self.static_empty_(f, run, p, node),
        }
    }

    #[inline(never)]
    fn static_empty_(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> bool {
        if cfg!(feature = "vec-step-clock") && sw!("VEC_STEP_TIME") && self.stats.static_depth == 0 {
            self.stats.static_depth = 1;
            let t0 = super::step_clock();
            let e = self.static_empty_(f, run, p, node);
            self.stats.static_time += super::step_clock() - t0;
            self.stats.static_depth = 0;
            return e;
        }
        let r = self.rec(run, node);
        if r.flags & CAN == 0 {
            f.known[node] = NO;
            return false;
        }
        f.known[node] = BUSY;
        self.stats.static_visits += 1;
        if r.first.is_empty() && r.flags & (FILE_TEST | EP_LEAF) == 0 {
            self.stats.static_pass += 1;
        }
        let mut e = self.any_static(f, run, p, r.first);
        if !e && r.flags & FILE_TEST != 0 {
            self.stats.static_file_tests += 1;
            e = filter_order::file_fails(run, f, &r.path);
        }
        let mut ep = false;
        if !e && r.n_ins > 0 {
            let (a, b) = (r.ins as usize, r.ins as usize + usize::from(r.n_ins));
            if r.flags & ALL != 0 {
                e = (a..b).all(|i| {
                    let x = self.flat.ins[i] as usize;
                    let y = self.static_empty(f, run, p, x);
                    ep |= y && f.known[x] == YES_EP;
                    y
                });
            } else {
                e = (a..b).any(|i| {
                    let x = self.flat.ins[i] as usize;
                    let y = self.static_empty(f, run, p, x);
                    ep = y && f.known[x] == YES_EP;
                    y
                });
            }
        }
        // redo-empty: the leaves the propagation adds (a fill with no rows, a literal no row of the file holds), after the
        // inputs: a fill is not asked where an input decides already
        if !e && r.flags & EP_LEAF != 0 {
            e = self.any_static(f, run, p, r.propagated);
            ep = e;
        }
        f.known[node] = if e && ep { YES_EP } else if e { YES } else { NO };
        e
    }

    pub(crate) fn bind_proofs(&mut self, run: &mut Run, emits: &[usize]) {
        // Compiling expressions can intern a literal. Hash-group iteration in
        // ordered and as-of uses those numeric IDs. Bind the proof now, while
        // keeping publication of runtime expression caches in first-demand order.
        let old: Vec<_> = (0..self.plan.nodes.len()).map(|i|
            (self.schemas[i].is_some(), self.views[i].is_some(), self.sels[i].is_some())
        ).collect();
        let words = run.texts.len();
        self.tests.initial_words = words;
        let paths = run.path_tests.checkpoint();
        self.flat.publication.resize(self.plan.nodes.len(), (0,0));
        let mut visited = vec![false; self.plan.nodes.len()];
        let mut pending: Vec<_> = emits.iter().map(|&e| self.plan.emits[e].1).collect();
        while let Some(node) = pending.pop() {
            if std::mem::replace(&mut visited[node], true) { continue; }
            self.rec(run, node);
            pending.extend_from_slice(&self.plan.nodes[node].inputs);
        }
        if run.texts.len() == words {
            self.flat.publication.clear();
            self.flat.publish_edges.clear();
            return;
        }
        for word in run.texts.drain(words..) { let _ = run.ids.remove(word.as_ref()); }
        run.path_tests.restore(paths);
        for (i, (schema, view, sel)) in old.into_iter().enumerate() {
            if !schema { self.schemas[i] = None; }
            if !view { self.views[i] = None; }
            if !sel { self.sels[i] = None; }
            if self.flat.recs[i].flags & READY != 0 { self.flat.recs[i].flags |= PUBLISH; }
        }
    }

    /// Reachable records are bound at load. Diagnostic entry points may also request another record.
    #[inline]
    fn rec(&mut self, run: &mut Run, node: usize) -> Rec {
        let r = self.flat.recs[node];
        if r.flags & (READY | PUBLISH) == READY {
            return r;
        }
        if r.flags & PUBLISH != 0 { return self.publish(run, node, r); }
        self.rec_of(run, node)
    }

    /// The shape and test IDs are already fixed. Expose the selectors that
    /// the old first request of this record compiled, in their original order.
    #[inline(never)]
    fn publish(&mut self, run: &mut Run, node: usize, mut r: Rec) -> Rec {
        r.flags &= !PUBLISH;
        self.flat.recs[node] = r;
        if !self.is_constant(run, node) {
            if let Ok(Some(view)) = self.view(run, node) {
                r.path = view.paths;
            } else if self.plan.nodes[node].op == "select" && let Ok(sel) = self.sel(run, node) {
                r.path = sel.paths;
                for &source in &sel.srcs { let _ = self.view(run, source); }
            }
            let (start,end) = self.flat.publication[node];
            for at in start..end {
                let source = self.flat.publish_edges[at as usize] as usize;
                let _ = self.rec(run, source);
            }
        }
        self.flat.recs[node] = r;
        r
    }

    #[inline(never)]
    fn rec_of(&mut self, run: &mut Run, node: usize) -> Rec {
        if cfg!(feature = "vec-step-clock") && sw!("VEC_STEP_TIME") && !self.stats.rec_busy {
            self.stats.rec_busy = true;
            let t0 = super::step_clock();
            let r = self.rec_of(run, node);
            self.stats.rec_time += super::step_clock() - t0;
            self.stats.rec_busy = false;
            return r;
        }
        // a cycle is impossible (a plan is a DAG); mark ready first all the same
        self.flat.recs[node] = Rec { flags: READY | NEVER, ..Rec::default() };
        let need = self.need_of(run, node);
        let mut r = Rec { flags: READY, ..Rec::default() };
        if need.never {
            r.flags |= NEVER;
            self.flat.recs[node] = r;
            return r;
        }
        if !self.flat.publication.is_empty() {
            let start = self.flat.publish_edges.len() as u32;
            self.flat.publish_edges.extend(need.inputs.iter().map(|&i| i as u32));
            self.flat.publication[node] = (start, self.flat.publish_edges.len() as u32);
        }
        let can_in: Vec<bool> = need.inputs.iter().map(|i| self.rec(run, *i).flags & CAN != 0).collect();
        let kept: Vec<u32> = if need.all {
            if !can_in.is_empty() && can_in.iter().all(|c| *c) { need.inputs.iter().map(|i| *i as u32).collect() } else { Vec::new() }
        } else {
            need.inputs.iter().zip(&can_in).filter(|(_, c)| **c).map(|(i, _)| *i as u32).collect()
        };
        r.ins = self.flat.ins.len() as u32;
        r.n_ins = kept.len() as u16;
        self.flat.ins.extend(kept);
        use super::static_tests::Test::{Kinds, Seed};
        r.first = self.tests.bind(need.kinds.into_vec().into_iter().map(Kinds)
            .chain(need.seeds.iter().copied().map(Seed)));
        if need.all {
            r.flags |= ALL;
        }
        if !need.path.is_empty() {
            r.flags |= FILE_TEST;
            r.path = need.path;
        }
        if !sw!("VEC_NO_EMPTYPROP") { r.propagated = self.bind_ep(run, node); }
        if !r.propagated.is_empty() {
            r.flags |= EP_LEAF;
        }
        if r.n_ins > 0 || !r.first.is_empty() || !need.path.is_empty() || r.flags & EP_LEAF != 0 {
            r.flags |= CAN;
        }
        self.flat.recs[node] = r;
        r
    }

    /// The step's needs, prepared at its first ask and kept for the process (one plan a process).
    #[inline]
    pub(super) fn need(&mut self, run: &mut Run, node: usize) -> &'static Need {
        if let Some(n) = self.needs[node] {
            return n;
        }
        let n: &'static Need = Box::leak(Box::new(self.need_of(run, node)));
        self.needs[node] = Some(n);
        n
    }

    fn need_of(&mut self, run: &mut Run, node: usize) -> Need {
        let mut n = Need { inputs: Box::new([]), all: false, kinds: Box::new([]), seeds: Box::new([]), path: Default::default(), never: false };
        if self.is_constant(run, node) {
            n.never = true;
            return n;
        }
        let codes = |run: &Run, syms: &[u32]| -> Box<[u16]> { run.kind_sym.iter().enumerate().filter(|(_, s)| syms.contains(s)).map(|(c, _)| c as u16).collect() };
        if let Ok(Some(v)) = self.view(run, node) {
            if v.base == Native::Tree {
                n.kinds = v.filter.iter().filter_map(|c| super::kind_set(run, std::slice::from_ref(c), 0, v.base, &[])).map(|ks| codes(run, &ks)).collect();
            }
            n.path = v.paths;
            n.seeds = v.filter.iter().filter_map(|e| if let E::Seed(_, k) = e { Some(*k) } else { None }).collect();
            return n;
        }
        let plan = self.plan;
        let pn = &plan.nodes[node];
        let first = match pn.op.as_str() {
            "select" => {
                let Ok(sp) = self.sel(run, node) else {
                    n.never = true;
                    return n;
                };
                let mut kinds = Vec::new();
                for (i, s) in sp.srcs.iter().enumerate() {
                    if let Ok(Some(v)) = self.view(run, *s)
                        && v.base == Native::Tree
                    {
                        kinds.extend(sp.conj.iter().filter_map(|c| super::kind_set(run, std::slice::from_ref(c), i, v.base, &v.map)).map(|ks| codes(run, &ks)));
                    }
                }
                n.kinds = kinds.into();
                n.seeds = sp.conj.iter().filter_map(|e| if let E::Seed(_, k) = e { Some(*k) } else { None }).collect();
                n.path = sp.paths;
                n.inputs = sp.srcs.clone().into();
                return n;
            }
            "union" => {
                n.inputs = pn.inputs.clone().into();
                n.all = true;
                return n;
            }
            // the counted group-count reads its select's kept side with zero counts: never known from its input
            "group-count" if pn.j["args"]["distinct"].is_null() && !sw!("VEC_NO_SEMI") && self.arg_rel(node, "from").is_ok_and(|x| plan.nodes[x].j.get("count").is_some()) => {
                n.never = true;
                return n;
            }
            "antijoin" | "intersection" | "difference" | "ordered-match" | "as-of" => "left",
            "closure" | "follow" => "edges",
            "base" | "rows" | "worklist" => {
                n.never = true;
                return n;
            }
            _ => "from",
        };
        match self.arg_rel(node, first) {
            Ok(i) => n.inputs = Box::new([i]),
            Err(_) => n.never = true,
        }
        n
    }
}
