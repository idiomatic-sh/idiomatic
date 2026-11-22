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
    /// Kind sets the rows must have one of, as masks over kind codes: empty when the file has none of a set.
    kinds: Box<[Box<[u64]>]>,
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

impl Exec<'_> {
    /// Whether a run of `node` in this file returns nothing, known without running it.
    #[inline(always)]
    pub(super) fn known_empty(&mut self, f: &mut File<'_>, run: &mut Run, node: usize) -> bool {
        let need = self.need(run, node);
        if need.never {
            return false;
        }
        // made empty already (read live)
        let made = |i: &usize| matches!(&f.memo[*i], Some(Res::Tab(t)) if t.n == 0);
        let live = if need.all { !need.inputs.is_empty() && need.inputs.iter().all(made) } else { need.inputs.iter().any(made) };
        live || self.static_empty(f, run, node)
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
        f.known[node] = BUSY;
        let need = self.need(run, node);
        if !need.kinds.is_empty() && f.kind_bits.is_empty() {
            // the file's kinds present, one bit a kind code (once a file)
            let mut bits = vec![0u64; run.kind_sym.len().div_ceil(64).max(1)];
            for (c, w) in (0..run.kind_sym.len()).map(|c| (c, c / 64)) {
                if !super::node_table::ids_of_kind(f.table, super::KindBits(c as u16)).is_empty() {
                    bits[w] |= 1 << (c % 64);
                }
            }
            f.kind_bits = bits;
        }
        let e = !need.never && {
            need.kinds.iter().any(|m| m.iter().zip(&f.kind_bits).all(|(a, b)| a & b == 0))
                || need.seeds.iter().any(|k| f.seeds.get(*k).is_none_or(Vec::is_empty))
                || (need.file_test && self.file_fails_of(f, run, node))
                || if need.all { !need.inputs.is_empty() && need.inputs.iter().all(|i| self.static_empty(f, run, *i)) } else { need.inputs.iter().any(|i| self.static_empty(f, run, *i)) }
        };
        f.known[node] = if e { YES } else { NO };
        e
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
        let codes = |run: &Run, syms: &[u32]| -> Box<[u64]> {
            let mut m = vec![0u64; run.kind_sym.len().div_ceil(64).max(1)];
            for (c, _) in run.kind_sym.iter().enumerate().filter(|(_, s)| syms.contains(s)) {
                m[c / 64] |= 1 << (c % 64);
            }
            m.into()
        };
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
