//! THROWAWAY (ryo-fmx3.1, step 9): `select` as two kernels, prepared once a plan node.
//!
//! Every select of the plan is one source filtered and projected, or two sources equi-joined on the columns the
//! condition equates, the other conjuncts a filter, then projected. A node is prepared once a run (its conjuncts as a
//! flat predicate over (source, column) with constants interned and operators resolved, its items as column positions);
//! an execution then reads columns by position into the worker's reused buffers.
//!
//! What the step reads and which side drives is decided exactly as the interpreter's `select` decides it (the front below
//! is that code, kept, so the same steps are made or probed and the rows and findings stay equal). `VEC_OLD_SELECT=1`
//! runs the interpreter's `select` instead (the oracle). A node with a form the kernels do not cover goes to the
//! interpreter's `select` too (`VEC_KSEL_LOG=<path>` lists each node's path once a run).

use super::{E, Exec, File, Native, Op, Provider, R, Run, Src, Table, Ty, V, NULL};
use super::{bucket, bucket_len, compare, distinct, eval, get, index_of, key_of, kind_set, kinds_required, lit_rows, refs_of, table_index, text_of, truthy, view_holds, view_rows};
use rustc_hash::FxHashSet;
use serde_json::Value as J;
use std::cell::RefCell;
use std::rc::Rc;

/// An operand of a predicate: a source's column, or a constant.
#[derive(Clone, Debug)]
enum Opd {
    Col(usize, usize, Ty),
    K(V),
}

/// A conjunct compiled once a node: comparisons over column positions, the interpreter's semantics exactly.
#[derive(Clone, Debug)]
enum P {
    /// Two columns of one type equal (neither null), or not (`want` false).
    EqCC(usize, usize, usize, usize, bool),
    /// A symbol column equal to an interned text, or not.
    EqCK(usize, usize, u64, bool),
    /// Any other comparison, by `compare`.
    Cmp(Op, Opd, Opd),
    In(Opd, Vec<V>, bool),
    Re(Opd, Rc<regex::bytes::Regex>),
    Seed(Opd, usize),
    And(Box<P>, Box<P>),
    Or(Box<P>, Box<P>),
    Not(Box<P>),
    Const(bool),
    /// A form with a function call: the interpreter's row evaluation.
    Old(E),
}

fn opd(e: &E) -> Option<Opd> {
    Some(match e {
        E::Col(s, c, t) => Opd::Col(*s, *c, *t),
        E::Lit(v) => Opd::K(*v),
        E::Text(t) => Opd::K(V::new(Ty::Sym, u64::from(*t))),
        E::Num(n) => Opd::K(V::new(Ty::Num, *n as u64)),
        _ => return None,
    })
}

fn compile(e: &E) -> P {
    match e {
        E::Lit(v) => P::Const(truthy(*v)),
        E::List(_) => P::Const(false),
        E::And(a, b) => P::And(Box::new(compile(a)), Box::new(compile(b))),
        E::Or(a, b) => P::Or(Box::new(compile(a)), Box::new(compile(b))),
        E::Not(a) => P::Not(Box::new(compile(a))),
        E::Cmp(o @ (Op::Eq | Op::Ne), a, b) => {
            let want = *o == Op::Eq;
            match (&**a, &**b) {
                (E::Col(s1, c1, t1), E::Col(s2, c2, t2)) if t1 == t2 => P::EqCC(*s1, *c1, *s2, *c2, want),
                (E::Col(s1, c1, Ty::Sym), E::Text(t)) | (E::Text(t), E::Col(s1, c1, Ty::Sym)) => P::EqCK(*s1, *c1, u64::from(*t), want),
                _ => match (opd(a), opd(b)) {
                    (Some(x), Some(y)) => P::Cmp(*o, x, y),
                    _ => P::Old(e.clone()),
                },
            }
        }
        E::Cmp(o, a, b) => match (opd(a), opd(b)) {
            (Some(x), Some(y)) => P::Cmp(*o, x, y),
            _ => P::Old(e.clone()),
        },
        E::In(a, b, neg) => {
            let items: Option<Vec<V>> = match &**b {
                E::List(items) => items.iter().map(|i| match opd(i) {
                    Some(Opd::K(v)) => Some(v),
                    _ => None,
                }).collect(),
                _ => Some(Vec::new()),
            };
            match (opd(a), items) {
                (Some(x), Some(items)) => P::In(x, items, *neg),
                _ => P::Old(e.clone()),
            }
        }
        E::Re(a, re) => match opd(a) {
            Some(x) => P::Re(x, re.clone()),
            None => P::Old(e.clone()),
        },
        E::Seed(a, k) => match opd(a) {
            Some(x) => P::Seed(x, *k),
            None => P::Old(e.clone()),
        },
        _ => P::Old(e.clone()),
    }
}

#[inline]
fn val(run: &mut Run, f: &File<'_>, srcs: &[Src], cur: &[u32], o: &Opd) -> V {
    match o {
        Opd::Col(s, c, t) => V::new(*t, get(run, f, srcs, cur, *s, *c)),
        Opd::K(v) => *v,
    }
}

fn holds(run: &mut Run, f: &File<'_>, srcs: &[Src], cur: &[u32], p: &P) -> bool {
    match p {
        P::EqCC(s1, c1, s2, c2, want) => {
            let (x, y) = (get(run, f, srcs, cur, *s1, *c1), get(run, f, srcs, cur, *s2, *c2));
            (x == y && x != NULL) == *want
        }
        P::EqCK(s, c, k, want) => (get(run, f, srcs, cur, *s, *c) == *k) == *want,
        P::Cmp(o, a, b) => {
            let (x, y) = (val(run, f, srcs, cur, a), val(run, f, srcs, cur, b));
            let Some(ord) = compare(run, x, y) else { return false };
            use std::cmp::Ordering::*;
            match o {
                Op::Eq => ord == Equal,
                Op::Ne => ord != Equal,
                Op::Lt => ord == Less,
                Op::Le => ord != Greater,
                Op::Gt => ord == Greater,
                Op::Ge => ord != Less,
                Op::Other => false,
            }
        }
        P::In(a, items, neg) => {
            let x = val(run, f, srcs, cur, a);
            items.iter().any(|y| compare(run, x, *y) == Some(std::cmp::Ordering::Equal)) != *neg
        }
        P::Re(a, re) => {
            let x = val(run, f, srcs, cur, a);
            match x.ty() {
                Ty::Sym => re.is_match(run.text(x.1 as u32)),
                _ => re.is_match(&text_of(run, x)),
            }
        }
        P::Seed(a, k) => {
            let x = val(run, f, srcs, cur, a);
            f.seeds.get(*k).is_some_and(|s| s.binary_search(&(x.1 as u32)).is_ok())
        }
        P::And(a, b) => holds(run, f, srcs, cur, a) && holds(run, f, srcs, cur, b),
        P::Or(a, b) => holds(run, f, srcs, cur, a) || holds(run, f, srcs, cur, b),
        P::Not(a) => !holds(run, f, srcs, cur, a),
        P::Const(b) => *b,
        P::Old(e) => truthy(eval(run, f, srcs, cur, e)),
    }
}

/// Keeps the rows `sel` of source `si` that hold `p`: a column against a constant or two columns of one made table in a
/// tight loop over the column, else the predicate a row.
fn filter(run: &mut Run, f: &File<'_>, srcs: &[Src], si: usize, p: &P, sel: &mut Vec<u32>) {
    if let Src::Tab(t, _) = &srcs[si] {
        match p {
            P::EqCK(s, c, k, want) if *s == si => {
                let col = &t.cols[*c][..];
                let (k, want) = (*k, *want);
                sel.retain(|r| (col[*r as usize] == k) == want);
                return;
            }
            P::EqCC(s1, c1, s2, c2, want) if *s1 == si && *s2 == si => {
                let (a, b) = (&t.cols[*c1][..], &t.cols[*c2][..]);
                let want = *want;
                sel.retain(|r| {
                    let (x, y) = (a[*r as usize], b[*r as usize]);
                    (x == y && x != NULL) == want
                });
                return;
            }
            _ => {}
        }
    }
    let mut cur = [0u32; 2];
    sel.retain(|r| {
        cur[si] = *r;
        holds(run, f, srcs, &cur, p)
    });
}

/// A select node, prepared once a run.
struct K {
    preds: Vec<P>,
    /// Whether a conjunct is constant over the file (the per-file test runs only then).
    file_const: bool,
    items: Vec<(usize, usize)>,
    /// One source, every item a column: a projection that drops columns is a set.
    set: bool,
    /// One source, no test, its columns picked in order under their own names: the answer is the source's table itself.
    ident: bool,
    /// By source, where it is a view: the kinds its own tests name, and the kinds the select's tests name of it (static).
    kinds: Vec<(Option<Vec<u32>>, Option<Vec<u32>>)>,
    entries: Vec<super::view_entry::Entry>,
    /// A semi step's probed side: the band is tried first.
    semi: Option<usize>,
    /// redo-select2: a run is decided by a view source whose entry is empty before anything is read (`ksel_zero`).
    zero: bool,
}

/// Keys of the per-file row lists a kernel keeps in the file's `vrows` (above any node number).
pub(super) const LIT_FILTER: usize = 1 << 56;
pub(super) const LIT_CONJ: usize = 2 << 56;
const ENTRY_KIND: usize = 3 << 56;
const ENTRY_LIT: usize = 4 << 56;
const ABSENT: usize = 1 << 60;

/// `lit_rows`, once a file under `key` (`ABSENT` marks a None).
pub(super) fn lit_file(f: &mut File<'_>, key: usize, conj: &[E], si: usize, base: super::Native, map: &[usize]) -> Option<Rc<Vec<u32>>> {
    if !matches!(base, Native::Fact(_)) || conj.is_empty() { return None; }
    if let Some(r) = f.vrows.get(&key) {
        return Some(r.clone());
    }
    if f.vrows.contains_key(&(key | ABSENT)) {
        return None;
    }
    // redo-alloc: the rows made into a kept list (recycle.rs)
    let mut hit = false;
    let rc = super::recycle::rows_with(|o| hit = super::lit_rows_into(f, conj, si, base, map, o));
    match hit {
        true => {
            f.vrows.insert(key, rc.clone());
            Some(rc)
        }
        false => {
            super::recycle::recycle_rows(rc);
            // redo-alloc: the run's one empty list marks it (no list made a mark)
            f.vrows.insert(key | ABSENT, super::recycle::no_rows());
            None
        }
    }
}

enum Prep {
    K(K),
    Old(&'static str),
}

/// Step 9 order of work: kernel 1 (one source) measured first, then kernel 2 switched on.
const KERNEL2: bool = true;

thread_local! {
    /// The prepared nodes (by the plan's address, then node), and the worker's buffers (a stack: executions nest).
    static PREPS: RefCell<(usize, Vec<Option<Rc<Prep>>>)> = const { RefCell::new((0, Vec::new())) };
    static BUFS: RefCell<Vec<Bufs>> = const { RefCell::new(Vec::new()) };
}

/// `VEC_KSEL_PROF`: seconds by section (whole kernel call exclusive of inner steps, front, driver filter, join, projection,
/// the interpreter's select exclusive), and the calls; written to `$VEC_KSEL_LOG.prof` every 2^16 calls.
/// The monotonic clock in seconds (`VEC_STEP_TICKS`): read through the vDSO, not a system call.
pub(super) fn ticks() -> f64 {
    static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    ORIGIN.get_or_init(std::time::Instant::now).elapsed().as_secs_f64()
}

#[derive(Clone, Copy)]
struct Clk(f64);
impl Clk {
    fn now() -> Clk {
        Clk(if sw!("VEC_KSEL_PROF") { super::step_clock() } else { 0.0 })
    }
    fn elapsed(self) -> f64 {
        Clk::now().0 - self.0
    }
}
static PROF: std::sync::Mutex<([f64; 8], u64)> = std::sync::Mutex::new(([0.0; 8], 0));
fn prof0() -> bool {
    sw!("VEC_KSEL_PROF")
}
fn prof_add(i: usize, dt: f64) {
    let mut p = PROF.lock().expect("prof");
    p.0[i] += dt;
    if i == 0 || i == 5 {
        p.1 += 1;
        if p.1 % (1 << 16) == 0
            && let Some(path) = std::env::var_os("VEC_KSEL_LOG")
        {
            let mut path = path;
            path.push(".prof");
            let _ = std::fs::write(path, format!("calls {} whole {:.3} front {:.3} filter {:.3} join {:.3} project {:.3} old {:.3} one-source tail {:.3} set-up {:.3}\n", p.1, p.0[0], p.0[1], p.0[2], p.0[3], p.0[4], p.0[5], p.0[6], p.0[7]));
        }
    }
}

#[derive(Default)]
struct Bufs {
    sel: Vec<u32>,
    left: Vec<u32>,
    right: Vec<u32>,
    hits: Vec<u32>,
    keys: Vec<u64>,
    seen: FxHashSet<u64>,
    key: Vec<u64>,
    /// A probed side's (key, row) pairs, sorted: the join's index for a table made for this execution only.
    pairs: Vec<(u64, u32)>,
    /// redo-alloc: the sources of one run (emptied after it), and its join's columns (the probed side's, the driver's)
    srcs: Vec<Src>,
    join_cols: (Vec<usize>, Vec<usize>),
}

fn take_bufs() -> Bufs {
    BUFS.with(|b| b.borrow_mut().pop()).unwrap_or_default()
}

fn give_bufs(mut b: Bufs) {
    // redo-select2: buffers kept up to 64 k entries (1 M kept up to 28 MB a worker past a large run)
    for v in [&mut b.sel, &mut b.left, &mut b.right, &mut b.hits] {
        if v.capacity() > 1 << 16 {
            *v = Vec::new();
        }
        v.clear();
    }
    b.keys.clear();
    b.key.clear();
    if b.pairs.capacity() > 1 << 16 {
        b.pairs = Vec::new();
    }
    b.pairs.clear();
    if b.seen.capacity() > 1 << 16 {
        b.seen = FxHashSet::default();
    }
    BUFS.with(|s| s.borrow_mut().push(b));
}

impl Exec<'_> {
    fn ksel_prep(&mut self, run: &mut Run, node: usize) -> R<Rc<Prep>> {
        let addr = std::ptr::from_ref(self.plan) as usize;
        if let Some(p) = PREPS.with(|c| {
            let c = c.borrow();
            if c.0 == addr { c.1.get(node).cloned().flatten() } else { None }
        }) {
            return Ok(p);
        }
        let prep = Rc::new(self.ksel_make(run, node)?);
        if let Some(path) = std::env::var_os("VEC_KSEL_LOG") {
            use std::io::Write;
            let why = match &*prep {
                Prep::K(k) if k.semi.is_some() => "semi (band, else kernel)",
                Prep::K(_) => "kernel",
                Prep::Old(w) => w,
            };
            if let Ok(mut o) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(o, "{node}\t{why}");
            }
        }
        PREPS.with(|c| {
            let mut c = c.borrow_mut();
            if c.0 != addr {
                *c = (addr, Vec::new());
            }
            if c.1.len() <= node {
                c.1.resize(node + 1, None);
            }
            c.1[node] = Some(prep.clone());
        });
        Ok(prep)
    }

    fn ksel_make(&mut self, run: &mut Run, node: usize) -> R<Prep> {
        let plan = self.plan;
        let j = &plan.nodes[node].j;
        // a semi step tries the band first (band.rs); where it declines, the kernels run it (`VEC_KSEL_NO_SEMI`: the old path)
        let semi = j.get("semi").and_then(J::as_u64).filter(|_| j.get("count").is_none() && !sw!("VEC_NO_SEMI")).map(|x| x as usize);
        if semi.is_some() && sw!("VEC_KSEL_NO_SEMI") {
            return Ok(Prep::Old("semi"));
        }
        let sp = self.sel(run, node)?;
        let k = sp.srcs.len();
        if k > 2 {
            return Ok(Prep::Old("three or more sources"));
        }
        if k == 2 && !KERNEL2 {
            return Ok(Prep::Old("two sources (kernel 2 not on)"));
        }
        if sp.srcs.iter().any(|n| self.plan.nodes[*n].op == "closure") {
            return Ok(Prep::Old("a closure source"));
        }
        let mut items = Vec::with_capacity(sp.items.len());
        for (_, e) in &sp.items {
            match e {
                E::Col(s, c, _) => items.push((*s, *c)),
                _ => return Ok(Prep::Old("a computed item")),
            }
        }
        // a join key the front can pick (an equality of a column of one source with an expression over the other) must be a column
        if k == 2 {
            for e in &sp.conj {
                if let E::Cmp(Op::Eq, a, b) = e
                    && refs_of(e) == 3
                    && (matches!(**a, E::Col(..)) || matches!(**b, E::Col(..)))
                    && !(matches!(**a, E::Col(..)) && matches!(**b, E::Col(..)))
                {
                    return Ok(Prep::Old("a join key that is not a column"));
                }
            }
        }
        let preds = sp.conj.iter().map(compile).collect();
        let file_const = sp.conj.iter().any(|c| super::filter_order::file_const(c) && !matches!(c, E::Lit(_)));
        let set = k == 1 && items.iter().all(|(s, _)| *s == 0);
        let ident = k == 1 && sp.conj.is_empty() && {
            let ssc = self.schema(run, sp.srcs[0])?;
            items.len() == ssc.names.len() && items.iter().enumerate().all(|(i, (s, c))| *s == 0 && *c == i) && sp.items.iter().zip(ssc.names.iter()).all(|((n, _), m)| n == m)
        };
        let mut kinds = Vec::with_capacity(k);
        let mut entries = Vec::with_capacity(k);
        for (i, n) in sp.srcs.iter().enumerate() {
            let view = self.view(run, *n)?;
            let sets = view.as_ref().map_or((None, None), |v| {
                (kind_set(run, &v.filter, 0, v.base, &[]), kind_set(run, &sp.conj, i, v.base, &v.map))
            });
            let selected = sets.0.as_deref().or(sets.1.as_deref());
            entries.push(view.as_ref().map_or(super::view_entry::Entry::Unknown, |v| {
                super::view_entry::Entry::bind(run, v, selected, &sp.conj, i)
            }));
            kinds.push(sets);
        }
        // an empty view entry empties the run whatever the other sources hold (every source is joined)
        let zero = semi.is_none() && !sp.conj.iter().any(|e| matches!(e, E::Seed(..))) && {
            let mut views = false;
            for s in sp.srcs.iter() {
                views |= self.view(run, *s)?.is_some();
            }
            views
        };
        Ok(Prep::K(K { preds, file_const, items, set, ident, kinds, entries, semi, zero }))
    }

    /// The dispatch: the kernels, or the interpreter's `select` (`VEC_OLD_SELECT`, or a node they do not cover).
    pub(super) fn select_k(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {
        if sw!("VEC_OLD_SELECT") {
            return self.select(f, run, p, node);
        }
        let prof = sw!("VEC_KSEL_PROF");
        let (t0, i0) = if prof { (Clk::now(), self.inner) } else { (Clk::now(), 0.0) };
        let prep = self.ksel_prep(run, node)?;
        let Prep::K(kp) = &*prep else {
            let out = self.select(f, run, p, node);
            if prof {
                prof_add(5, t0.elapsed() - (self.inner - i0));
            }
            return out;
        };
        if let Some(pb) = kp.semi {
            let sp = self.sel(run, node)?;
            if super::filter_order::file_fails(run, f, &sp.conj) {
                *self.tally.back.entry((node, "a per-file test fails")).or_default() += 1;
                return self.empty(run, node);
            }
            if let Some(t) = self.semi(f, run, p, node, pb)? {
                return Ok(t);
            }
        }
        let mut b = take_bufs();
        let out = self.ksel_run(f, run, p, node, kp, &mut b, None);
        give_bufs(b);
        if prof {
            prof_add(0, t0.elapsed() - (self.inner - i0));
        }
        out.map(|t| Rc::try_unwrap(t).unwrap_or_else(|t| (*t).clone()))
    }

    /// redo-select2: whether a view source's entry (the size the front measures it by) is empty in this file, for a node
    /// whose run reads nothing else first (`K::zero`: not semi, no seed test, every source a view or a keyed step). The
    /// front would read the views, find that entry the smallest, drive from it and end. Only views
    /// whose fill is read already are measured (an unread fill is not read here; `lit_rows` would read it as absent).
    fn ksel_zero(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, kp: &K, sp: &super::SelPlan) -> bool {
        for (i, s) in sp.srcs.iter().enumerate() {
            if matches!(f.memo[*s], Some(super::Res::Tab(_))) {
                continue;
            }
            let Ok(Some(v)) = self.view(run, *s) else { continue };
            if let super::Native::Fact(x) = v.base
                && f.facts.get(usize::from(x)).is_none_or(Option::is_none)
                && !(super::lazy_view::owed(f, usize::from(x)) && self.load_fact(f, run, p, v.base).is_ok())
            {
                continue;
            }
            let size = match &kp.kinds[i] {
                (Some(ks), _) | (None, Some(ks)) => Some(bucket_len(f, run, ks)),
                (None, None) => lit_file(f, LIT_FILTER | *s, &v.filter, 0, v.base, &[]).or_else(|| lit_file(f, LIT_CONJ | (node << 1) | i, &sp.conj, i, v.base, &v.map)).map(|r| r.len()),
            };
            if size == Some(0) {
                return true;
            }
        }
        false
    }

    /// redo-select2: the kernels' entry for a whole run (from `empty_sel::select_entry`): the step's shared table out.
    pub(super) fn select_kr(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
        let prep = self.ksel_prep(run, node)?;
        let Prep::K(kp) = &*prep else {
            let t = self.select(f, run, p, node)?;
            return if t.n == 0 { self.empty_rc(run, node) } else { Ok(super::recycle::rc(t)) };
        };
        if let Some(pb) = kp.semi {
            let sp = self.sel(run, node)?;
            if super::filter_order::file_fails(run, f, &sp.conj) {
                *self.tally.back.entry((node, "a per-file test fails")).or_default() += 1;
                super::empty_sel::at(super::empty_sel::stage::FILE_TEST);
                return self.empty_rc(run, node);
            }
            if let Some(t) = self.semi(f, run, p, node, pb)? {
                super::empty_sel::at(super::empty_sel::stage::SEMI);
                return if t.n == 0 { self.empty_rc(run, node) } else { Ok(super::recycle::rc(t)) };
            }
        }
        let mut b = take_bufs();
        let out = self.ksel_run(f, run, p, node, kp, &mut b, None);
        give_bufs(b);
        out
    }

    /// redo-whole pass 2 (keyed.rs): the source of a two-source select read by the other's keys in place of made whole:
    /// (source, its column, the driver's column) of an equality of the two, the source a step whose one consumer is this
    /// select, not made in this file and read by key on that column. Not the source `not` (read by key already).
    fn ksel_deferred(&mut self, f: &File<'_>, run: &mut Run, sp: &super::SelPlan, not: Option<usize>) -> R<Option<(usize, usize, usize)>> {
        if sp.srcs.len() != 2 || sp.srcs[0] == sp.srcs[1] || sp.conj.iter().any(|e| matches!(e, E::Seed(..))) || sw!("VEC_NO_KEYED_JOIN") {
            return Ok(None);
        }
        for i in [1usize, 0] {
            if not == Some(i) || f.memo[sp.srcs[i]].is_some() {
                continue;
            }
            let o = 1 - i;
            for e in &sp.conj {
                let E::Cmp(Op::Eq, x, y) = e else { continue };
                for (a, bb) in [(x, y), (y, x)] {
                    if let (E::Col(sa, c, _), E::Col(sb, dc, _)) = (&**a, &**bb)
                        && *sa == i
                        && *sb == o
                        && self.keyed_ok(f, run, sp.srcs[i], *c)?
                    {
                        return Ok(Some((i, *c, *dc)));
                    }
                }
            }
        }
        Ok(None)
    }

    /// redo-whole pass 2 (keyed.rs): whether the kernels run `node` (a form they cover).
    pub(super) fn ksel_covers(&mut self, run: &mut Run, node: usize) -> R<bool> {
        Ok(matches!(&*self.ksel_prep(run, node)?, Prep::K(_)))
    }

    /// redo-whole pass 2 (keyed.rs): the select read by key: its source `si` read by `keys` on its column `c` (the
    /// output column asked) drives the run; (answer, whether it is that source's answer handed on). None: not a kernel.
    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    pub(super) fn select_keyed(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, si: usize, c: usize, keys: &[u64]) -> R<Option<(Rc<Table>, bool)>> {
        let prep = self.ksel_prep(run, node)?;
        let Prep::K(kp) = &*prep else { return Ok(None) };
        let mut b = take_bufs();
        let out = self.ksel_run(f, run, p, node, kp, &mut b, Some((si, c, keys)));
        give_bufs(b);
        let out = out?;
        let handed = out.n == 0 || kp.ident;
        Ok(Some((out, handed)))
    }

    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    fn ksel_run(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, kp: &K, b: &mut Bufs, keyed: Option<(usize, usize, &[u64])>) -> R<Rc<Table>> {
        // redo-alloc: the sources' list is the worker's (reused across runs); its tables given back after the run
        let mut srcs = std::mem::take(&mut b.srcs);
        srcs.clear();
        let mut jc = std::mem::take(&mut b.join_cols);
        let out = self.ksel_run_(f, run, p, node, kp, b, &mut srcs, &mut jc, keyed);
        for s in srcs.drain(..) {
            if let Src::Tab(t, _) = s {
                super::recycle::recycle(t);
            }
        }
        b.srcs = srcs;
        b.join_cols = jc;
        out
    }

    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    fn ksel_run_(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, kp: &K, b: &mut Bufs, srcs: &mut Vec<Src>, jc: &mut (Vec<usize>, Vec<usize>), keyed: Option<(usize, usize, &[u64])>) -> R<Rc<Table>> {
        let (t0, i0) = (Clk::now(), self.inner);
        super::empty_sel::origin(self.inner);
        let sp = self.sel(run, node)?;
        if kp.file_const && super::filter_order::file_fails(run, f, &sp.conj) {
            *self.tally.back.entry((node, "a per-file test fails")).or_default() += 1;
            super::empty_sel::at(super::empty_sel::stage::FILE_TEST);
            return self.empty_rc(run, node);
        }
        if kp.zero && !sw!("VEC_ESEL_NO_ZERO") && !sw!("VEC_OLD_EMPTYSEL") && self.ksel_zero(f, run, p, node, kp, &sp) {
            super::empty_sel::at(super::empty_sel::stage::ZERO_ENTRY);
            if super::empty_sel::on() && sw!("VEC_ESEL_NODES") {
                super::empty_sel::count(Box::leak(format!("zero node {node} srcs {:?}", sp.srcs).into_boxed_str()), 0.0);
            }
            return self.empty_rc(run, node);
        }
        let k = sp.srcs.len();
        let conj = &sp.conj;
        let refs = &sp.refs;
        // ---- the front: the interpreter's, kept (which inputs are made or probed, which side drives)
        if prof0() {
            prof_add(7, t0.elapsed());
        }
        super::empty_sel::lap(0, self.inner);
        // redo-fills pass 2: views the front reads no size of, filled only past the front's early ends (lazy_view.rs);
        // `srcs` is redo-alloc's kept list (the wrapper's)
        let mut late: [Option<Native>; 4] = [None; 4];
        // redo-whole pass 2 (keyed.rs): the source read by key (this run is itself by key), and the source read by the
        // driver's keys (a step whose one consumer is this join, on an equality with the other source)
        let mut deferred = if keyed.is_none() || k == 2 { self.ksel_deferred(f, run, &sp, keyed.map(|x| x.0))? } else { None };
        // its size (0: the join is empty, nothing else read)
        let dsize = match deferred {
            Some((i, _, _)) => self.keyed_size(f, run, p, sp.srcs[i], false)?,
            None => 0,
        };
        if deferred.is_some() && dsize == 0 {
            return self.empty_rc(run, node);
        }
        // the sources are made in order and an empty one ends the run: a deferred first source whose other side is not
        // made yet and not a view would have the other made first, so it is made in its turn instead
        if let Some((i, _, _)) = deferred
            && i == 0
            && f.memo[sp.srcs[1]].is_none()
            && self.view(run, sp.srcs[1])?.is_none()
        {
            deferred = None;
        }
        for (i, n) in sp.srcs.iter().enumerate() {
            if let Some((si, c, keys)) = keyed
                && si == i
            {
                let t = self.probe(f, run, p, *n, c, keys)?;
                if t.n == 0 {
                    return self.empty_rc(run, node);
                }
                srcs.push(Src::Tab(t, usize::MAX - i));
                continue;
            }
            if deferred.is_some_and(|d| d.0 == i) {
                srcs.push(Src::Base(Native::Tree));
                continue;
            }
            let src = match if i < 4 { self.lazy_view(f, run, *n, conj, i, kp.kinds[i].0.is_some() || kp.kinds[i].1.is_some())? } else { None } {
                Some(v) => {
                    late[i] = Some(v.base);
                    Src::View(v, *n)
                }
                None => self.src_of(f, run, p, *n)?,
            };
            if let Src::Tab(t, _) = &src
                && t.n == 0
            {
                super::empty_sel::with(|m| {
                    m.stage = super::empty_sel::stage::SOURCE_EMPTY;
                    m.src = i as u8;
                    m.srcn = *n as u32;
                });
                return self.empty_rc(run, node);
            }
            srcs.push(src);
        }
        super::empty_sel::lap(1, self.inner);
        // the deferred source is read by key only when the other side drives with fewer rows than its size; else made
        if let Some((i, _, _)) = deferred {
            let o = 1 - i;
            // the other side is made and not empty: the deferred source is read either way, so its inputs are made for its size
            let dsize = self.keyed_size(f, run, p, sp.srcs[i], true)?;
            let osize = match &srcs[o] {
                Src::Tab(t, _) => Some(t.n),
                Src::View(..) => kp.entries[o].len(f),
                _ => None,
            };
            if osize.is_none_or(|n| n >= dsize) {
                deferred = None;
                let src = self.src_of(f, run, p, sp.srcs[i])?;
                if let Src::Tab(t, _) = &src
                    && t.n == 0
                {
                    return self.empty_rc(run, node);
                }
                srcs[i] = src;
            }
        }
        if kp.ident
            && let Src::Tab(t, _) = &srcs[0]
        {
            return Ok(t.clone());
        }
        let mut driver: Option<usize> = keyed.map(|x| x.0).or(deferred.map(|d| 1 - d.0));
        if let Some((si, c, sk)) = conj.iter().find_map(|e| match e {
            E::Seed(x, sk) => match **x {
                E::Col(si, c, _) => Some((si, c, *sk)),
                _ => None,
            },
            _ => None,
        }) {
            let ids: Vec<u64> = f.seeds.get(sk).map(|s| s.iter().map(|x| u64::from(*x)).collect()).unwrap_or_default();
            let t = self.probe(f, run, p, sp.srcs[si], c, &ids)?;
            self.stats.touched[node] += ids.len() as u64;
            if t.n == 0 {
                super::empty_sel::at(super::empty_sel::stage::SEED_EMPTY);
                return self.empty_rc(run, node);
            }
            srcs[si] = Src::Tab(t, usize::MAX - si);
            driver = Some(si);
        }
        super::empty_sel::lap(2, self.inner);
        let mut best = if driver.is_some() { 0 } else { usize::MAX };
        for (i, sr) in srcs.iter().enumerate() {
            let size = match sr {
                Src::Tab(t, _) => Some(t.n),
                Src::View(..) => kp.entries[i].len(f),
                _ => None,
            };
            if let Some(n) = size
                && n < best
            {
                best = n;
                driver = Some(i);
            }
        }
        super::empty_sel::lap(3, self.inner);
        for b in late.into_iter().flatten() {
            self.load_fact(f, run, p, b)?;
        }
        let driver = match driver {
            Some(d) => d,
            None => (0..k).find(|i| matches!(srcs[*i], Src::View(..))).ok_or("a select with no source to start from")?,
        };
        super::empty_sel::lap(5, self.inner);
        let prof = sw!("VEC_KSEL_PROF");
        if prof {
            prof_add(1, t0.elapsed() - (self.inner - i0));
        }
        let t1 = Clk::now();
        // ---- kernel 1: the driver's rows, filtered by its own conjuncts one at a time over a selection vector
        let dmask = 1u64 << driver;
        let mut used: u64 = 0;
        let sel = &mut b.sel;
        sel.clear();
        match &srcs[driver] {
            Src::Tab(t, _) => {
                self.stats.touched[node] += t.n as u64;
                super::empty_sel::with(|m| m.d0 = t.n as u32);
                sel.extend(0..t.n as u32);
            }
            Src::View(v, vn) => {
                // the entry rows, once a file a (step, driver): the select's kind bucket or literal rows of the view, tested
                let first: super::cold_bucket::Rows = match &kp.kinds[driver] {
                    (None, Some(ks)) => match f.vrows.get(&(ENTRY_KIND | (node << 1) | driver)) {
                        Some(r) => r.clone().into(),
                        None => {
                            let rc: Rc<Vec<u32>> = {
                                let mut rows = bucket(f, run, ks);
                                super::view_cols::retain(f, run, v, &mut rows);
                                Rc::new(rows)
                            };
                            f.vrows.insert(ENTRY_KIND | (node << 1) | driver, rc.clone());
                            rc.into()
                        }
                    },
                    _ => match lit_file(f, LIT_CONJ | (node << 1) | driver, conj, driver, v.base, &v.map) {
                        Some(rows) if lit_file(f, LIT_FILTER | *vn, &v.filter, 0, v.base, &[]).is_none() => match f.vrows.get(&(ENTRY_LIT | (node << 1) | driver)) {
                            Some(r) => r.clone().into(),
                            None => {
                                let rc: Rc<Vec<u32>> = {
                                    let mut kept = rows.to_vec();
                                    super::view_cols::retain(f, run, v, &mut kept);
                                    Rc::new(kept)
                                };
                                f.vrows.insert(ENTRY_LIT | (node << 1) | driver, rc.clone());
                                rc.into()
                            }
                        },
                        _ => view_rows(f, run, v, *vn),
                    },
                };
                self.stats.touched[node] += first.len() as u64;
                super::empty_sel::with(|m| m.d0 = first.len() as u32);
                first.extend_into(sel);
            }
            _ => return Err("a select starts from no table or view".to_owned()),
        }
        for (ci, pr) in kp.preds.iter().enumerate() {
            if refs[ci] & !dmask == 0 {
                used |= 1 << ci;
                if !sel.is_empty() {
                    filter(run, f, &srcs, driver, pr, sel);
                }
            }
        }
        if prof {
            prof_add(2, t1.elapsed());
        }
        super::empty_sel::lap(6, self.inner);
        super::empty_sel::with(|m| {
            m.d1 = sel.len() as u32;
            m.stage = if k == 1 { super::empty_sel::stage::ONE_SOURCE_FILTERED } else { super::empty_sel::stage::DRIVER_FILTERED };
        });
        let t2 = Clk::now();
        let i2 = self.inner;
        let all: u64 = if conj.len() >= 64 { u64::MAX } else { (1u64 << conj.len()) - 1 };
        if k == 1 {
            let t6 = Clk::now();
            let out = self.ksel_one(run, f, node, kp, &srcs, sel, used == all, &mut b.seen, &mut b.left, &mut b.right)?;
            if prof {
                prof_add(6, t6.elapsed());
            }
            return if out.n == 0 { self.empty_rc(run, node) } else { Ok(super::recycle::rc(out)) };
        }
        // ---- kernel 2: the other source, joined on the columns the conjuncts equate
        if sel.is_empty() {
            return self.empty_rc(run, node);
        }
        if let Some((i, c, dc)) = deferred {
            // the deferred source read by the driver's kept rows' keys (distinct)
            let keys = &mut b.keys;
            keys.clear();
            let mut cur = [0u32; 2];
            for r in sel.iter() {
                cur[driver] = *r;
                keys.push(get(run, f, srcs, &cur, driver, dc));
            }
            keys.sort_unstable();
            keys.dedup();
            let keys = std::mem::take(keys);
            let t = self.probe(f, run, p, sp.srcs[i], c, &keys);
            b.keys = keys;
            let t = t?;
            if t.n == 0 {
                return self.empty_rc(run, node);
            }
            srcs[i] = Src::Tab(t, usize::MAX - i);
        }
        let other = 1 - driver;
        let omask = 1u64 << other;
        // the join column: an unused equality of the other side's column with a column of the driver, one probed in place first
        let mut pick: Option<(usize, usize, usize, bool)> = None;
        for (ci, c) in conj.iter().enumerate() {
            if used & (1 << ci) != 0 {
                continue;
            }
            if let E::Cmp(Op::Eq, x, y) = c {
                for (a, bb) in [(x, y), (y, x)] {
                    if let E::Col(si, col, _) = **a
                        && si == other
                        && refs_of(bb) == dmask
                    {
                        let native = match &srcs[si] {
                            Src::View(v, _) => File::probes(v.base, v.map[col]),
                            _ => false,
                        };
                        if pick.is_none_or(|q| native && !q.3) {
                            pick = Some((si, col, ci, native));
                        }
                    }
                }
            }
        }
        super::empty_sel::lap(7, self.inner);
        let left = &mut b.left;
        let right = &mut b.right;
        left.clear();
        right.clear();
        let mut cur = [0u32; 2];
        match pick {
            Some((si, col, ci, native)) => {
                used |= 1 << ci;
                let (dc, dty) = match &conj[ci] {
                    E::Cmp(_, x, y) => match (&**x, &**y) {
                        (E::Col(a, c1, _), E::Col(_, c2, t2)) if *a == si && *c1 == col => (*c2, *t2),
                        (E::Col(_, c1, t1), _) => (*c1, *t1),
                        _ => return Err("a join key that is not a column".to_owned()),
                    },
                    _ => unreachable!(),
                };
                let _ = dty;
                if let Src::View(_, vn) = &srcs[si]
                    && !native
                {
                    let vn = *vn;
                    srcs[si] = Src::Tab(self.eval(f, run, p, vn)?, vn);
                }
                // a made table is probed on every column an equality binds, at once
                // redo-alloc: the join's columns in the worker's two buffers
                let (tab_cols, drv_cols) = (&mut jc.0, &mut jc.1);
                tab_cols.clear();
                drv_cols.clear();
                tab_cols.push(col);
                drv_cols.push(dc);
                if let Src::Tab(..) = &srcs[si] {
                    for (cj, c) in conj.iter().enumerate() {
                        if used & (1 << cj) != 0 {
                            continue;
                        }
                        if let E::Cmp(Op::Eq, x, y) = c {
                            for (a2, b2) in [(x, y), (y, x)] {
                                if let E::Col(s3, c3, _) = **a2
                                    && s3 == si
                                    && refs_of(b2) == dmask
                                    && !tab_cols.contains(&c3)
                                {
                                    let E::Col(_, c4, _) = **b2 else { return Err("a join key that is not a column".to_owned()) };
                                    tab_cols.push(c3);
                                    drv_cols.push(c4);
                                    break;
                                }
                            }
                        }
                    }
                }
                if tab_cols.len() > 1 {
                    used &= !(1 << ci);
                }
                let nb = dmask | omask;
                // redo-alloc: the tests left, on the stack (a select has fewer than 64: `used` is a word)
                let mut ckb = [0usize; 64];
                let mut nck = 0;
                for c in (0..conj.len()).filter(|c| used & (1 << c) == 0 && refs[*c] & !nb == 0) {
                    ckb[nck] = c;
                    nck += 1;
                }
                let checks = &ckb[..nck];
                let hits = &mut b.hits;
                let mut touched = 0u64;
                super::empty_sel::with(|m| {
                    m.o = match &srcs[si] {
                        Src::Tab(t, _) => t.n as u32,
                        _ => u32::MAX,
                    };
                    m.built |= match &srcs[si] {
                        Src::Tab(_, tn) if *tn > usize::MAX - 64 => super::empty_sel::stage::B_INDEX_FRESH,
                        Src::Tab(..) => super::empty_sel::stage::B_INDEX_FILE,
                        _ => super::empty_sel::stage::B_VIEW_PROBE,
                    };
                });
                match &srcs[si] {
                    Src::Tab(t, tn) => {
                        // a table made for this execution (probed, or put in place) is indexed by sorting its (key, row)
                        // pairs in the worker's buffer and searched by halving; a step's own table by its index, once a file
                        let fresh = *tn > usize::MAX - 64;
                        let ix = if fresh && !sw!("VEC_KSEL_INDEX") {
                            let pairs = &mut b.pairs;
                            pairs.clear();
                            if tab_cols.len() == 1 {
                                let c = &t.cols[tab_cols[0]];
                                pairs.extend((0..t.n).map(|r| (c[r], r as u32)));
                            } else {
                                pairs.extend((0..t.n).map(|r| (key_of(tab_cols.iter().map(|c| t.cols[*c][r])), r as u32)));
                            }
                            pairs.sort_unstable();
                            None
                        } else if fresh {
                            Some(Rc::new(index_of(t, &tab_cols)))
                        } else {
                            Some(table_index(f, *tn, t, &tab_cols))
                        };
                        let pairs = &b.pairs;
                        // redo-alloc: the driver's table read by its key columns in place (no list of them made a run)
                        let dtab: Option<&Table> = match &srcs[driver] {
                            Src::Tab(dt, _) => Some(dt),
                            _ => None,
                        };
                        for r in sel.iter() {
                            let r = *r;
                            cur[driver] = r;
                            let key = match dtab {
                                Some(dt) if drv_cols.len() == 1 => dt.cols[drv_cols[0]][r as usize],
                                Some(dt) => key_of(drv_cols.iter().map(|c| dt.cols[*c][r as usize])),
                                None => {
                                    b.key.clear();
                                    for c in drv_cols.iter() {
                                        b.key.push(get(run, f, &srcs, &cur, driver, *c));
                                    }
                                    key_of(b.key.iter().copied())
                                }
                            };
                            let rows: &[u32] = match &ix {
                                Some(ix) => match ix.get(&key) {
                                    Some(rows) => rows,
                                    None => continue,
                                },
                                None => {
                                    let a = pairs.partition_point(|x| x.0 < key);
                                    let e = a + pairs[a..].partition_point(|x| x.0 == key);
                                    if a == e {
                                        continue;
                                    }
                                    hits.clear();
                                    hits.extend(pairs[a..e].iter().map(|x| x.1));
                                    &hits[..]
                                }
                            };
                            touched += rows.len() as u64;
                            if checks.is_empty() {
                                for h in rows {
                                    left.push(r);
                                    right.push(*h);
                                }
                            } else {
                                for h in rows {
                                    cur[si] = *h;
                                    if checks.iter().all(|c| holds(run, f, &srcs, &cur, &kp.preds[*c])) {
                                        left.push(r);
                                        right.push(*h);
                                    }
                                }
                            }
                        }
                    }
                    Src::View(vs, _) => {
                        let (base, bcol) = (vs.base, vs.map[col]);
                        let batch = super::view_probe::Driver { rows: sel, source: driver, column: dc };
                        let direct = super::view_probe::pairs(f, run, &srcs, batch, vs, bcol, |run, row, hit| {
                            cur[driver] = row;
                            cur[si] = hit;
                            if checks.iter().all(|c| holds(run, f, &srcs, &cur, &kp.preds[*c])) {
                                left.push(row);
                                right.push(hit);
                            }
                        });
                        if let Some(count) = direct {
                            touched += count;
                        } else {
                            for r in sel.iter() {
                                let r = *r;
                                cur[driver] = r;
                                let v = get(run, f, &srcs, &cur, driver, dc);
                                hits.clear();
                                f.native_probe(run, base, bcol, v, hits);
                                super::view_cols::retain(f, run, vs, hits);
                                touched += hits.len() as u64;
                                for h in hits.iter() {
                                    cur[si] = *h;
                                    if checks.iter().all(|c| holds(run, f, &srcs, &cur, &kp.preds[*c])) {
                                        left.push(r);
                                        right.push(*h);
                                    }
                                }
                            }
                        }
                    }
                    _ => return Err("a join side that is no table or view".to_owned()),
                }
                self.stats.touched[node] += touched;
                super::empty_sel::at(if touched > 0 { super::empty_sel::stage::RESIDUAL } else { super::empty_sel::stage::KEYS_MISS });
                for c in checks {
                    used |= 1 << c;
                }
            }
            None => {
                // no key joins the other source: every pair
                super::empty_sel::at(super::empty_sel::stage::CROSS);
                let nb = dmask | omask;
                // redo-alloc: the tests left, on the stack (a select has fewer than 64: `used` is a word)
                let mut ckb = [0usize; 64];
                let mut nck = 0;
                for c in (0..conj.len()).filter(|c| used & (1 << c) == 0 && refs[*c] & !nb == 0) {
                    ckb[nck] = c;
                    nck += 1;
                }
                let checks = &ckb[..nck];
                let rows: super::cold_bucket::Rows = match &srcs[other] {
                    Src::Tab(t, _) => Rc::new((0..t.n as u32).collect::<Vec<_>>()).into(),
                    Src::View(v, vn) => view_rows(f, run, v, *vn),
                    _ => return Err("a join side that is no table or view".to_owned()),
                };
                for r in sel.iter() {
                    cur[driver] = *r;
                    self.stats.touched[node] += rows.len() as u64;
                    for h in rows.iter() {
                        cur[other] = *h;
                        if checks.iter().all(|c| holds(run, f, &srcs, &cur, &kp.preds[*c])) {
                            left.push(*r);
                            right.push(*h);
                        }
                    }
                }
                for c in checks {
                    used |= 1 << c;
                }
            }
        }
        super::empty_sel::lap(8, self.inner);
        if used != all {
            return Err("a test no source binds".to_owned());
        }
        let t3 = Clk::now();
        if prof {
            prof_add(3, t2.elapsed() - (self.inner - i2));
        }
        let n = left.len();
        // redo-alloc: the columns taken with the join's room
        let mut out = self.empty_rows(run, node, n)?;
        for (c, (si, sc)) in kp.items.iter().enumerate() {
            let rows: &[u32] = if *si == driver { left } else { right };
            let col = &mut out.cols[c];
            col.reserve(n);
            match &srcs[*si] {
                Src::Tab(t, _) => {
                    let src = &t.cols[*sc][..];
                    col.extend(rows.iter().map(|r| src[*r as usize]));
                }
                Src::View(v, _) => {
                    let (base, bc) = (v.base, v.map[*sc]);
                    super::view_cols::gather(f, run, base, bc, &rows, col);
                }
                _ => return Err("a select reads no table or view".to_owned()),
            }
        }
        out.n = n;
        if prof {
            prof_add(4, t3.elapsed());
        }
        if n == 0 {
            return self.empty_rc(run, node);
        }
        Ok(super::recycle::rc(out))
    }

    /// Kernel 1's tail: the kept rows projected by position; a projection that drops columns kept as a set (first
    /// occurrences, in row order), deduplicated in the worker's buffers.
    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    fn ksel_one(&mut self, run: &mut Run, f: &File<'_>, node: usize, kp: &K, srcs: &[Src], sel: &[u32], all_used: bool, seen: &mut FxHashSet<u64>, idx: &mut Vec<u32>, kept: &mut Vec<u32>) -> R<Table> {
        if !all_used {
            return Err("a test no source binds".to_owned());
        }
        let mut out = self.empty(run, node)?;
        let n = sel.len();
        for (c, (_, sc)) in kp.items.iter().enumerate() {
            let col = &mut out.cols[c];
            col.reserve(n);
            match &srcs[0] {
                Src::Tab(t, _) => {
                    let src = &t.cols[*sc][..];
                    col.extend(sel.iter().map(|r| src[*r as usize]));
                }
                Src::View(v, _) => {
                    let (base, bc) = (v.base, v.map[*sc]);
                    super::view_cols::gather(f, run, base, bc, &sel, col);
                }
                _ => return Err("a select reads no table or view".to_owned()),
            }
        }
        out.n = n;
        let set = kp.set
            && !sw!("VEC_SELECT_BAG")
            && match &srcs[0] {
                Src::Tab(t, _) => kp.items.len() < t.names.len(),
                Src::View(v, _) => kp.items.len() < v.map.len(),
                _ => false,
            };
        if !set || n < 2 || sw!("VEC_UNION_ALL") {
            return Ok(out);
        }
        if sw!("VEC_KSEL_DISTINCT") {
            return Ok(distinct(out));
        }
        kept.clear();
        if out.cols.len() == 1 {
            seen.clear();
            let c = &out.cols[0];
            kept.extend((0..n as u32).filter(|r| seen.insert(c[*r as usize])));
        } else {
            idx.clear();
            idx.extend(0..n as u32);
            let cols = &out.cols;
            idx.sort_unstable_by(|a, b| {
                for c in cols {
                    match c[*a as usize].cmp(&c[*b as usize]) {
                        std::cmp::Ordering::Equal => {}
                        o => return o,
                    }
                }
                a.cmp(b)
            });
            let same = |a: u32, b: u32| cols.iter().all(|c| c[a as usize] == c[b as usize]);
            for (i, r) in idx.iter().enumerate() {
                if i == 0 || !same(idx[i - 1], *r) {
                    kept.push(*r);
                }
            }
            kept.sort_unstable();
        }
        if kept.len() == n {
            return Ok(out);
        }
        let mut d = self.empty(run, node)?;
        for (c, col) in out.cols.iter().enumerate() {
            d.cols[c].extend(kept.iter().map(|r| col[*r as usize]));
        }
        d.n = kept.len();
        Ok(d)
    }
}
