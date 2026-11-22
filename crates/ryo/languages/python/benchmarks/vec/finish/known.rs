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

use super::{E, Exec, File, Native, Res, Run, filter_order};

/// What a step needs to return anything, prepared once a run (at its first ask).
pub(super) struct Need {
    /// Empty when any of these is empty (or, `all`, when every one is).
    inputs: Box<[usize]>,
    all: bool,
    /// Kind sets the rows must have one of (kind codes): empty when the file has none of a set.
    kinds: Box<[Box<[u16]>]>,
    /// Survivor sets a seed test reads.
    seeds: Box<[usize]>,
    /// A per-file test to decide.
    file_test: bool,
    /// Never known empty (a constant step: made once a run, as before).
    never: bool,
}

const NO: u8 = 1;
const YES: u8 = 2;
const BUSY: u8 = 3;

/// pass 2: a step's needs in flat arrays (one record a step, its lists in shared arrays), for the per-file decisions:
/// the inputs kept are those that can be known empty themselves (`CAN`); a step none of whose needs can is never
/// decided a file.
#[derive(Clone, Copy, Default)]
pub(super) struct Rec {
    ins: u32,
    n_ins: u16,
    n_kinds: u16,
    kinds: u32,
    seeds: u32,
    n_seeds: u16,
    flags: u8,
}
const READY: u8 = 1;
const NEVER: u8 = 2;
const ALL: u8 = 4;
const FILE_TEST: u8 = 8;
const CAN: u8 = 16;

#[derive(Default)]
pub(super) struct Flat {
    recs: Vec<Rec>,
    ins: Vec<u32>,
    /// Kind sets: a length, then that many kind codes.
    kinds: Vec<u16>,
    seeds: Vec<u16>,
}

impl Flat {
    pub(super) fn new(nodes: usize) -> Self {
        Flat { recs: vec![Rec::default(); nodes], ..Flat::default() }
    }
}

impl Exec<'_> {
    /// Whether a run of `node` in this file returns nothing, known without running it.
    #[inline(always)]
    pub(super) fn known_empty(&mut self, f: &mut File<'_>, run: &mut Run, node: usize) -> bool {
        match f.known[node] {
            YES => return true,
            NO | BUSY => {}
            _ => {
                if self.static_empty_(f, run, node) {
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
    fn static_empty(&mut self, f: &mut File<'_>, run: &mut Run, node: usize) -> bool {
        match f.known[node] {
            NO | BUSY => false,
            YES => true,
            _ => self.static_empty_(f, run, node),
        }
    }

    #[inline(never)]
    fn static_empty_(&mut self, f: &mut File<'_>, run: &mut Run, node: usize) -> bool {
        if sw!("VEC_STEP_TIME") && self.stats.static_depth == 0 {
            self.stats.static_depth = 1;
            let t0 = super::step_clock();
            let e = self.static_empty_(f, run, node);
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
        if !e && r.n_seeds > 0 {
            e = self.flat.seeds[r.seeds as usize..r.seeds as usize + usize::from(r.n_seeds)].iter().any(|k| f.seeds.get(usize::from(*k)).is_none_or(Vec::is_empty));
        }
        if !e && r.flags & FILE_TEST != 0 {
            self.stats.static_file_tests += 1;
            e = self.file_fails_of(f, run, node);
        }
        if !e && r.n_ins > 0 {
            let (a, b) = (r.ins as usize, r.ins as usize + usize::from(r.n_ins));
            if r.flags & ALL != 0 {
                e = (a..b).all(|i| {
                    let x = self.flat.ins[i] as usize;
                    self.static_empty(f, run, x)
                });
            } else {
                e = (a..b).any(|i| {
                    let x = self.flat.ins[i] as usize;
                    self.static_empty(f, run, x)
                });
            }
        }
        f.known[node] = if e { YES } else { NO };
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
            if !can_in.is_empty() && can_in.iter().all(|c| *c) { need.inputs.iter().map(|i| *i as u32).collect() } else { Vec::new() }
        } else {
            need.inputs.iter().zip(&can_in).filter(|(_, c)| **c).map(|(i, _)| *i as u32).collect()
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
        r.seeds = self.flat.seeds.len() as u32;
        self.flat.seeds.extend(need.seeds.iter().map(|k| *k as u16));
        r.n_seeds = need.seeds.len() as u16;
        if need.all {
            r.flags |= ALL;
        }
        if need.file_test {
            r.flags |= FILE_TEST;
        }
        if r.n_ins > 0 || r.n_kinds > 0 || r.n_seeds > 0 || need.file_test {
            r.flags |= CAN;
        }
        self.flat.recs[node] = r;
        r
    }

    /// The step's needs, prepared at its first ask and kept for the process (one plan a process).
    #[inline]
    fn need(&mut self, run: &mut Run, node: usize) -> &'static Need {
        if let Some(n) = self.needs[node] {
            return n;
        }
        let n: &'static Need = Box::leak(Box::new(self.need_of(run, node)));
        self.needs[node] = Some(n);
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
        let mut n = Need { inputs: Box::new([]), all: false, kinds: Box::new([]), seeds: Box::new([]), file_test: false, never: false };
        if self.is_constant(node) {
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
            "group-count" if pn.j["args"]["distinct"].is_null() && !sw!("VEC_NO_SEMI") && self.arg_rel(node, "from").is_ok_and(|x| plan.nodes[x].j.get("count").is_some()) => {
                n.never = true;
                return n;
            }
            "antijoin" | "ordered-match" | "as-of" => "left",
            "closure" | "follow" => "edges",
            "base" | "rows" => {
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
