//! THROWAWAY (ryo-fmx3.1, redo-select2): `Exec::bound_size` for the select kernels, the same answer at a fraction of the
//! cost. The interpreter's bound walks the plan below a pending source (to depth 6) on every select run, matching
//! operator names, looking up arguments and computing each view's bound (its kind bucket, else its literal rows)
//! anew. Here the walk's shape is prepared once a node in flat arrays (operator code, inputs resolved), a view's bound
//! is kept once a file (a view's rows are fixed by the file; a probe's substitution never puts a view in place), and a
//! question set (the pendsize pick of one select run: nothing is made between its questions) answers each (node, depth)
//! once. A made table's rows are read live from the file's memo, as before. A view whose filter fixes no column to
//! literals has no bound, and its fill is not read for one (`lit_rows` would answer None whatever the fill holds).

use super::{E, Exec, File, Native, Provider, R, Res, Run, bucket_len, kind_set};
use std::cell::RefCell;

const UNKNOWN: u8 = 0;
const VIEW: u8 = 1;
const SELECT: u8 = 2;
const UNION: u8 = 3;
const UNARY: u8 = 4;
const OTHER: u8 = 5;

#[derive(Default)]
struct Ctx {
    plan: usize,
    /// By node: the shape's code, and its inputs (`a`, `b`: a union's two, a unary step's one; a select's in `ins`
    /// from `a`, `b` of them).
    code: Vec<u8>,
    a: Vec<u32>,
    b: Vec<u32>,
    ins: Vec<u32>,
    /// By node, a view's kind set (index into `kinds`, else u32::MAX) and whether its filter has a literal test.
    vkinds: Vec<u32>,
    vlit: Vec<bool>,
    kinds: Vec<Vec<u32>>,
    /// The views' bounds in this file, by node: 0 not known, 1 none, n + 2 a bound of n.
    file: (u32, usize),
    vb: Vec<u32>,
    /// The answers of this question set by node * 8 + depth: (set, answer + 1; 0 none).
    set: u32,
    found: Vec<(u32, u32)>,
}

thread_local! {
    static CTX: RefCell<Ctx> = RefCell::new(Ctx::default());
}

/// The columns of source 0 an expression names, as `lit_rows` walks them.
fn cols(e: &E, out: &mut Vec<usize>) {
    match e {
        E::Col(0, c, _) => out.push(*c),
        E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) | E::In(a, b, _) => {
            cols(a, out);
            cols(b, out);
        }
        _ => {}
    }
}

const LIT_FILTER: usize = 1 << 56;

impl Exec<'_> {
    /// The pendsize pick of a select run: among `pending` (source position, step), the first with the smallest bound
    /// under `best` (the kernel's loop over `bound_size`, exactly).
    pub(super) fn pendsize_pick(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, pending: &[(usize, usize)], best: usize) -> R<Option<(usize, usize, usize)>> {
        let mut pick: Option<(usize, usize, usize)> = None;
        if sw!("VEC_OLD_BOUND") || sw!("VEC_OLD_EMPTYSEL") {
            for &(i, n) in pending {
                if let Some(bs) = self.bound_size(f, run, p, n, 6)?
                    && bs < best
                    && pick.is_none_or(|q| bs < q.2)
                {
                    pick = Some((i, n, bs));
                }
            }
            return Ok(pick);
        }
        let mut cx = CTX.with(|c| std::mem::take(&mut *c.borrow_mut()));
        let addr = std::ptr::from_ref(self.plan) as usize;
        if cx.plan != addr {
            cx = Ctx { plan: addr, ..Ctx::default() };
        }
        let key = (f.file, std::ptr::from_ref(f.table) as usize);
        if cx.file != key {
            cx.file = key;
            cx.vb.clear();
        }
        cx.set = cx.set.wrapping_add(1);
        let tq = super::step_clock();
        let mut out = Ok(());
        for &(i, n) in pending {
            match self.bound_in(&mut cx, f, run, p, n, 6) {
                Ok(Some(bs)) if bs < best && pick.is_none_or(|q| bs < q.2) => pick = Some((i, n, bs)),
                Ok(_) => {}
                Err(e) => {
                    out = Err(e);
                    break;
                }
            }
        }
        CTX.with(|c| *c.borrow_mut() = cx);
        if super::empty_sel::on() {
            super::empty_sel::count("bound: question set", super::step_clock() - tq);
        }
        out.map(|()| pick)
    }

    fn bound_prep(&mut self, cx: &mut Ctx, run: &mut Run, node: usize) -> R<()> {
        if cx.code.len() <= node {
            let n = self.plan.nodes.len().max(node + 1);
            cx.code.resize(n, UNKNOWN);
            cx.a.resize(n, 0);
            cx.b.resize(n, 0);
            cx.vkinds.resize(n, u32::MAX);
            cx.vlit.resize(n, false);
        }
        if let Some(v) = self.view(run, node)? {
            let lit = matches!(v.base, Native::Fact(_))
                && v.filter.iter().any(|e| {
                    let mut cs = Vec::new();
                    cols(e, &mut cs);
                    cs.iter().any(|c| super::kinds_required(e, 0, *c).is_some())
                });
            if let Some(ks) = kind_set(run, &v.filter, 0, v.base, &[]) {
                cx.vkinds[node] = cx.kinds.len() as u32;
                cx.kinds.push(ks);
            }
            cx.vlit[node] = lit;
            cx.code[node] = VIEW;
            return Ok(());
        }
        let pn = &self.plan.nodes[node];
        cx.code[node] = match pn.op.as_str() {
            "select" => {
                cx.a[node] = cx.ins.len() as u32;
                cx.ins.extend(pn.inputs.iter().map(|i| *i as u32));
                cx.b[node] = pn.inputs.len() as u32;
                SELECT
            }
            "union" => {
                cx.a[node] = self.arg_rel(node, "left")? as u32;
                cx.b[node] = self.arg_rel(node, "right")? as u32;
                UNION
            }
            "antijoin" => {
                cx.a[node] = self.arg_rel(node, "left")? as u32;
                UNARY
            }
            "extend" | "project" => {
                cx.a[node] = self.arg_rel(node, "from")? as u32;
                UNARY
            }
            _ => OTHER,
        };
        Ok(())
    }

    /// `bound_size`, exactly: a made table's rows, a view's bucket or literal rows, a select's smallest bounded input, a
    /// union's sum, an antijoin's left, a pass-through step's input; None: unknown.
    fn bound_in(&mut self, cx: &mut Ctx, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, depth: u32) -> R<Option<usize>> {
        if let Some(Res::Tab(t)) = &f.memo[node] {
            return Ok(Some(t.n));
        }
        if cx.code.get(node).is_none_or(|c| *c == UNKNOWN) {
            self.bound_prep(cx, run, node)?;
        }
        let code = cx.code[node];
        if code == VIEW {
            return self.view_bound(cx, f, run, p, node);
        }
        if depth == 0 {
            return Ok(None);
        }
        let at = node * 8 + depth as usize;
        if let Some(e) = cx.found.get(at)
            && e.0 == cx.set
        {
            return Ok(e.1.checked_sub(1).map(|x| x as usize));
        }
        let b = match code {
            SELECT => {
                let (a, n) = (cx.a[node] as usize, cx.b[node] as usize);
                let mut m: Option<usize> = None;
                for j in a..a + n {
                    let i = cx.ins[j] as usize;
                    if let Some(b) = self.bound_in(cx, f, run, p, i, depth - 1)? {
                        m = Some(m.map_or(b, |x| x.min(b)));
                    }
                }
                m
            }
            UNION => {
                let (l, r) = (cx.a[node] as usize, cx.b[node] as usize);
                match (self.bound_in(cx, f, run, p, l, depth - 1)?, self.bound_in(cx, f, run, p, r, depth - 1)?) {
                    (Some(a), Some(b)) => Some(a + b),
                    _ => None,
                }
            }
            UNARY => {
                let i = cx.a[node] as usize;
                self.bound_in(cx, f, run, p, i, depth - 1)?
            }
            _ => None,
        };
        if cx.found.len() <= at {
            cx.found.resize(at + 8, (0, 0));
        }
        cx.found[at] = (cx.set, b.map_or(0, |x| x as u32 + 1));
        Ok(b)
    }

    fn view_bound(&mut self, cx: &mut Ctx, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Option<usize>> {
        let ki = cx.vkinds[node];
        if ki == u32::MAX && !cx.vlit[node] {
            return Ok(None);
        }
        if let Some(&k) = cx.vb.get(node)
            && k != 0
        {
            return Ok((k >= 2).then(|| (k - 2) as usize));
        }
        let Some(v) = self.view(run, node)? else { return Err("a view's bound with no view".to_owned()) };
        let t0 = super::step_clock();
        self.load_fact(f, run, p, v.base)?;
        if super::empty_sel::on() {
            super::empty_sel::count("bound: a literal view's fill read", super::step_clock() - t0);
        }
        let b = if ki != u32::MAX { Some(bucket_len(f, run, &cx.kinds[ki as usize])) } else { super::select_kernel::lit_file(f, LIT_FILTER | node, &v.filter, 0, v.base, &[]).map(|r| r.len()) };
        if cx.vb.len() <= node {
            cx.vb.resize(node + 1, 0);
        }
        cx.vb[node] = b.map_or(1, |n| n as u32 + 2);
        Ok(b)
    }
}
