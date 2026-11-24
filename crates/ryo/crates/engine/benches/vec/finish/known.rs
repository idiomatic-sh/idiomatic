//! THROWAWAY (ryo-fmx3.1, redo-union pass 2): a step run known to return nothing before it is entered. What the file
//! already knows decides it, with no table, no stack push and no clock:
//! - its kind buckets: a view, or a select's tree source, whose test names kinds none of which the file has;
//! - a per-file test that fails (the path tests `filter_order` decides);
//! - an input the step needs: any source of a select, the first input of every other step (the left of an antijoin,
//!   the from of a pass-through step, a closure's edges), every input of a union: known empty by the above, or made
//!   empty already in this file.
//!
//! Exactly the cases where the step's body returns nothing before reading anything else (`Exec::body`, `select`). The
//! static part (kinds and file tests, through the inputs) is kept a file; the made part is read live (a probe's
//! substitution changes what is made, never what the file holds).

use super::{E, Exec, File, Native, Provider, Res, Run, filter_order};

/// What a step needs to return anything, prepared once a run (at its first ask).
pub(super) struct Need {
    /// Empty when any of these is empty (or, `all`, when every one is).
    pub(super) inputs: Box<[usize]>,
    pub(super) all: bool,
    /// Kind sets the rows must have one of (kind codes): empty when the file has none of a set.
    pub(super) kinds: Box<[Box<[u16]>]>,
    /// A per-file test to decide.
    pub(super) file_test: bool,
    /// Never known empty (a constant step: made once a run, as before).
    pub(super) never: bool,
}

const NO: u8 = 1;
const YES: u8 = 2;
const BUSY: u8 = 3;
/// redo-empty: known empty by the propagation's own leaves (directly or through an input): counted apart.
pub(super) const YES_EP: u8 = 4;

/// pass 2: a step's needs in flat arrays (one record a step, its lists in shared arrays), for the per-file decisions:
/// the inputs kept are those that can be known empty themselves (`CAN`); a step none of whose needs can is never
/// decided a file.
#[derive(Clone, Copy, Default)]
pub(super) struct Rec {
    ins: u32,
    n_ins: u16,
    n_kinds: u16,
    kinds: u32,
    flags: u8,
}
const READY: u8 = 1;
const NEVER: u8 = 2;
const ALL: u8 = 4;
const FILE_TEST: u8 = 8;
const CAN: u8 = 16;
/// redo-empty: the step has a leaf of the propagation (`emptyprop`).
const EP_LEAF: u8 = 32;
/// redo-whole pass 2: no leaf of its own, empty when any input is (its inputs stand in for it in its consumers' lists).
const PASS: u8 = 64;
/// redo-whole pass 2: no leaf of its own, empty when every input is (its inputs stand in for it in such a consumer).
const AND: u8 = 128;

#[derive(Default)]
pub(super) struct Flat {
    recs: Vec<Rec>,
    ins: Vec<u32>,
    /// Kind sets: a length, then that many kind codes.
    kinds: Vec<u16>,
}

impl Flat {
    pub(super) fn new(nodes: usize) -> Self {
        Flat { recs: vec![Rec::default(); nodes], ..Flat::default() }
    }
}

impl Exec<'_> {
    /// Whether a run of `node` in this file returns nothing, known without running it.
    #[inline(always)]
    pub(super) fn known_empty(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> bool {
        if f.after_last_file { return false; }
        match f.known[node] {
            YES | YES_EP => return true,
            NO | BUSY => {}
            _ => {
                // redo-whole pass 2: a step empty only when an input is, every input made in this file and not empty, is
                // not empty by its inputs (a made table with rows was not empty by them either): no visit
                let r = self.flat.recs[node];
                if r.flags & (READY | PASS) == READY | PASS
                    && self.needs[node].as_ref().is_some_and(|n| n.inputs.iter().all(|i| matches!(&f.memo[*i], Some(Res::Tab(t)) if t.n > 0)))
                {
                    f.known[node] = NO;
                    self.stats.static_made_skips += 1;
                    return false;
                }
                self.stats.static_asks += 1;
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
        if f.after_last_file { return false; }
        match f.known[node] {
            NO | BUSY => false,
            YES | YES_EP => true,
            _ => self.static_empty_(f, run, p, node),
        }
    }

    #[inline(never)]
    fn static_empty_(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> bool {
        if sw!("VEC_STEP_TIME") && self.stats.static_depth == 0 {
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
        if r.n_kinds == 0 && r.flags & (FILE_TEST | EP_LEAF) == 0 {
            self.stats.static_pass += 1;
            if r.flags & ALL != 0 {
                self.stats.static_all += 1;
            }
        }
        if r.n_kinds > 0 && f.kind_bits.is_empty() {
            // the file's kinds present, one bit a kind code (once a file)
            let mut bits = vec![0u64; run.kind_sym.len().div_ceil(64).max(1)];
            for (c, w) in (0..run.kind_sym.len()).map(|c| (c, c / 64)) {
                if !super::node_table::ids_of_kind(f.table, super::KindBits(c as u16)).is_empty() {
                    bits[w] |= 1 << (c % 64);
                }
            }
            f.kind_bits = bits;
        }
        let mut e = false;
        // a kind set none of whose kinds the file has
        let (mut at, end) = (r.kinds as usize, r.kinds as usize + usize::from(r.n_kinds));
        while !e && at < end {
            let n = usize::from(self.flat.kinds[at]);
            e = self.flat.kinds[at + 1..at + 1 + n].iter().all(|c| f.kind_bits[usize::from(*c) / 64] & (1 << (c % 64)) == 0);
            at += 1 + n;
        }
        if !e && r.flags & FILE_TEST != 0 {
            self.stats.static_file_tests += 1;
            e = self.file_fails_of(f, run, node);
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
            e = self.ep_leaf_empty(f, run, p, node);
            ep = e;
        }
        f.known[node] = if e && ep { YES_EP } else if e { YES } else { NO };
        e
    }

    /// The step's flat record (made at its first ask, its inputs' first).
    #[inline]
    fn rec(&mut self, run: &mut Run, node: usize) -> Rec {
        let r = self.flat.recs[node];
        if r.flags & READY != 0 {
            return r;
        }
        self.rec_of(run, node)
    }

    #[inline(never)]
    fn rec_of(&mut self, run: &mut Run, node: usize) -> Rec {
        if sw!("VEC_STEP_TIME") && !self.stats.rec_busy {
            self.stats.rec_busy = true;
            let t0 = super::step_clock();
            let r = self.rec_of(run, node);
            self.stats.rec_time += super::step_clock() - t0;
            self.stats.rec_busy = false;
            return r;
        }
        // a cycle is impossible (a plan is a DAG); mark ready first all the same
        self.flat.recs[node] = Rec { flags: READY | NEVER, ..Rec::default() };
        let need = self.need(run, node);
        let mut r = Rec { flags: READY, ..Rec::default() };
        if need.never {
            r.flags |= NEVER;
            self.flat.recs[node] = r;
            return r;
        }
        let can_in: Vec<bool> = need.inputs.iter().map(|i| self.rec(run, *i).flags & CAN != 0).collect();
        let kept: Vec<u32> = if need.all {
            if !can_in.is_empty() && can_in.iter().all(|c| *c) {
                let kept: Vec<u32> = need.inputs.iter().map(|i| *i as u32).collect();
                // redo-whole pass 2: an input that is itself empty only when all of its inputs are, with no leaf of its
                // own (`AND`), is replaced by them; one empty when its one input is (`PASS`, one input) by that input
                self.flat_all(&kept)
            } else {
                Vec::new()
            }
        } else {
            let kept: Vec<u32> = need.inputs.iter().zip(&can_in).filter(|(_, c)| **c).map(|(i, _)| *i as u32).collect();
            // redo-whole pass 2: an input with no leaf of its own that is empty when any of its inputs is (`PASS`) is
            // replaced by its own (flattened) inputs, so a file's decision visits only steps that test something
            self.flat_inputs(&kept)
        };
        r.ins = self.flat.ins.len() as u32;
        r.n_ins = kept.len() as u16;
        self.flat.ins.extend(kept);
        r.kinds = self.flat.kinds.len() as u32;
        for ks in need.kinds.iter() {
            self.flat.kinds.push(ks.len() as u16);
            self.flat.kinds.extend(ks.iter().copied());
        }
        r.n_kinds = (self.flat.kinds.len() - r.kinds as usize) as u16;
        if need.all {
            r.flags |= ALL;
        }
        if need.file_test {
            r.flags |= FILE_TEST;
        }
        if !sw!("VEC_NO_EMPTYPROP") && self.ep_has_leaf(run, node) {
            r.flags |= EP_LEAF;
        }
        if r.n_ins > 0 || r.n_kinds > 0 || need.file_test || r.flags & EP_LEAF != 0 {
            r.flags |= CAN;
        }
        if r.flags & CAN != 0 && r.n_kinds == 0 && r.flags & (ALL | FILE_TEST | EP_LEAF) == 0 {
            r.flags |= PASS;
        }
        if r.flags & CAN != 0 && r.n_kinds == 0 && r.flags & ALL != 0 && r.flags & (FILE_TEST | EP_LEAF) == 0 {
            r.flags |= AND;
        }
        self.flat.recs[node] = r;
        r
    }

    /// The inputs a step's decision visits: each input, or for an input marked `PASS` its own visited inputs (flattened
    /// already: an input's record is made before its consumer's), once each, in order.
    fn flat_inputs(&self, kept: &[u32]) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::with_capacity(kept.len());
        let mut push = |x: u32, out: &mut Vec<u32>| {
            if !out.contains(&x) {
                out.push(x);
            }
        };
        for &x in kept {
            let r = self.flat.recs[x as usize];
            if r.flags & PASS != 0 {
                for &y in &self.flat.ins[r.ins as usize..r.ins as usize + usize::from(r.n_ins)] {
                    push(y, &mut out);
                }
            } else {
                push(x, &mut out);
            }
        }
        if out.len() > usize::from(u16::MAX) { kept.to_vec() } else { out }
    }

    /// `flat_inputs` for a step empty when every input is.
    fn flat_all(&self, kept: &[u32]) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::with_capacity(kept.len());
        for &x in kept {
            let mut x = x;
            // a chain of one-input steps with no leaf of their own: the last
            loop {
                let r = self.flat.recs[x as usize];
                if r.flags & PASS != 0 && r.n_ins == 1 {
                    x = self.flat.ins[r.ins as usize];
                } else {
                    break;
                }
            }
            let r = self.flat.recs[x as usize];
            if r.flags & AND != 0 {
                for &y in &self.flat.ins[r.ins as usize..r.ins as usize + usize::from(r.n_ins)] {
                    if !out.contains(&y) {
                        out.push(y);
                    }
                }
            } else if !out.contains(&x) {
                out.push(x);
            }
        }
        if out.len() > usize::from(u16::MAX) { kept.to_vec() } else { out }
    }

    /// The step's needs, prepared on its first use in this execution.
    #[inline]
    pub(super) fn need(&mut self, run: &mut Run, node: usize) -> std::rc::Rc<Need> {
        if let Some(n) = &self.needs[node] {
            return n.clone();
        }
        let n = std::rc::Rc::new(self.need_of(run, node));
        self.needs[node] = Some(n.clone());
        n
    }

    fn file_fails_of(&mut self, f: &File<'_>, run: &mut Run, node: usize) -> bool {
        if let Ok(Some(v)) = self.view(run, node) {
            return filter_order::file_fails(run, f, &v.filter);
        }
        match self.sel(run, node) {
            Ok(sp) => filter_order::file_fails(run, f, &sp.conj),
            Err(_) => false,
        }
    }

    fn need_of(&mut self, run: &mut Run, node: usize) -> Need {
        let mut n = Need { inputs: Box::new([]), all: false, kinds: Box::new([]), file_test: false, never: false };
        if self.is_constant(run, node) {
            n.never = true;
            return n;
        }
        let codes = |run: &Run, syms: &[u32]| -> Box<[u16]> { run.kind_sym.iter().enumerate().filter(|(_, s)| syms.contains(s)).map(|(c, _)| c as u16).collect() };
        let file_test = |conj: &[E]| conj.iter().any(|c| filter_order::file_const(c) && !matches!(c, E::Lit(_)));
        if let Ok(Some(v)) = self.view(run, node) {
            if v.base == Native::Tree {
                n.kinds = v.filter.iter().filter_map(|c| super::kind_set(run, std::slice::from_ref(c), 0, v.base, &[])).map(|ks| codes(run, &ks)).collect();
            }
            n.file_test = file_test(&v.filter);
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
                n.file_test = file_test(&sp.conj);
                n.inputs = sp.srcs.clone().into();
                return n;
            }
            "union" => {
                n.inputs = pn.inputs.clone().into();
                n.all = true;
                return n;
            }
            // the counted group-count reads its select's kept side with zero counts: never known from its input
            "group-count" if !pn.distinct_count() && !sw!("VEC_NO_SEMI") && self.arg_rel(node, "from").is_ok_and(|x| plan.nodes[x].counted()) => {
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
