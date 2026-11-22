//! THROWAWAY (ryo-fmx3.1, redo-union): the antijoin step as a kernel. Prepared once a run per step: the key columns on
//! both sides, whether the right side is a view probed in place on one of its keys (a knock-out per left row, nothing
//! made) or a table read by the left's keys (a set of its keys), and whether a probe of the step may run over the probed
//! left alone. A call that knocks out no row hands the left on (by key: itself, no copy); one that knocks out all hands
//! on the prepared empty table; else the kept rows are gathered into exact-size columns.

use super::kernel_input::{give_keys, input_probe, sole, take_keys};
use super::{Exec, File, Provider, R, Res, Run, Table, Ty, view_holds};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) struct Prep {
    left: usize,
    right: usize,
    lk: Box<[usize]>,
    rk: Box<[usize]>,
    /// The right key columns that hold the file (always equal within a file in the view path).
    rfile: Box<[bool]>,
    /// The right side is a view with a keyed read on this key.
    view_key: Option<usize>,
    /// The right side's key read by the left's keys when it is pending.
    rk_at: Option<usize>,
    /// A probe of the step runs over the probed left (no other input reads it, the left is read once).
    direct: bool,
    left_sole: bool,
    /// The right side is read by this step alone and the step is never probed (made whole once a file): the right is
    /// asked its keys once, so its per-key memo never answers.
    right_sole: bool,
    empty: Rc<Table>,
}

#[derive(Default)]
struct Work {
    keep: Vec<u32>,
    hits: Vec<u32>,
    set: FxHashSet<u64>,
    head: FxHashMap<u64, u32>,
    next: Vec<u32>,
}

thread_local! {
    /// Prepared once a run and kept for the process (one plan a process): read by a plain reference, no count.
    static PREP: RefCell<Vec<Option<&'static Prep>>> = const { RefCell::new(Vec::new()) };
    static WORK: RefCell<Work> = RefCell::new(Work::default());
}

/// Right sides this many rows or fewer are scanned, not hashed.
const SCAN: usize = 8;
const NONE: u32 = u32::MAX;

fn prep(ex: &mut Exec<'_>, run: &mut Run, node: usize) -> R<&'static Prep> {
    if let Some(Some(pr)) = PREP.with(|m| m.borrow().get(node).copied()) {
        return Ok(pr);
    }
    let plan = ex.plan;
    let a = &plan.nodes[node].j["args"];
    let (left, right) = (ex.arg_rel(node, "left")?, ex.arg_rel(node, "right")?);
    let (lsc, rsc) = (ex.schema(run, left)?, ex.schema(run, right)?);
    let lk: Box<[usize]> = super::strs(&a["left-on"])?.iter().map(|c| super::pos(&lsc.names, c)).collect::<R<_>>()?;
    let rk: Box<[usize]> = super::strs(&a["right-on"])?.iter().map(|c| super::pos(&rsc.names, c)).collect::<R<_>>()?;
    let rfile = rk.iter().map(|c| rsc.tys[*c] == Ty::File).collect();
    let view_key = match ex.view(run, right)? {
        Some(v) => (0..rk.len()).find(|i| File::probes(v.base, v.map[rk[*i]])),
        None => None,
    };
    let rk_at = (0..rk.len()).find(|i| rsc.tys[rk[*i]] != Ty::File).or(if ex.static_keyed { Some(0) } else { None });
    let inputs = plan.nodes[node].inputs.clone();
    let direct = inputs.iter().filter(|i| **i == left).count() == 1 && inputs.iter().filter(|i| **i != left).all(|o| !ex.depends(*o, left));
    let sc = ex.schema(run, node)?;
    let pr: &'static Prep = Box::leak(Box::new(Prep {
        left,
        right,
        lk,
        rk,
        rfile,
        view_key,
        rk_at,
        direct,
        left_sole: left != right && sole(ex, left, &[node]),
        right_sole: left != right && !ex.keyed(run, node) && sole(ex, right, &[node]),
        empty: Rc::new(Table::with(sc.names.clone(), sc.tys.clone())),
    }));
    PREP.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() <= node {
            m.resize(ex.plan.nodes.len().max(node + 1), None);
        }
        m[node] = Some(pr);
    });
    Ok(pr)
}

/// The antijoin made whole (body of the step; its left is not empty: the body returned before).
pub(super) fn whole(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
    let pr = prep(ex, run, node)?;
    let l = ex.eval(f, run, p, pr.left)?;
    if l.n == 0 {
        return Ok(pr.empty.clone());
    }
    over(ex, f, run, p, node, pr, &l)
}

/// The antijoin's rows whose column `col` holds one of `keys` (the static keyed read: the left probed, the step run over
/// it; else made whole and its rows taken).
#[expect(clippy::too_many_arguments, reason = "a throwaway")]
pub(super) fn probe(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64]) -> R<Rc<Table>> {
    let pr = prep(ex, run, node)?;
    if !pr.direct {
        ex.reason = "probed on a column its input shares with another input";
        let t = ex.eval(f, run, p, node)?;
        return Ok(super::recycle::rc(super::subset(f, node, &t, col, keys)));
    }
    let l = input_probe(ex, f, run, p, pr.left, col, keys, pr.left_sole)?;
    if l.n == 0 {
        return Ok(pr.empty.clone());
    }
    over(ex, f, run, p, node, &pr, &l)
}

/// The left rows no right row matches on every key.
fn over(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, pr: &Prep, l: &Rc<Table>) -> R<Rc<Table>> {
    if let Some(ki) = pr.view_key
        && let Res::View(v) = ex.res(f, run, p, pr.right)?
    {
        let (lk, rk) = (&pr.lk, &pr.rk);
        let mut touched = 0u64;
        let out = WORK.with(|w| {
            let w = &mut *w.borrow_mut();
            w.keep.clear();
            let probe_col = v.map[rk[ki]];
            let lcol = &l.cols[lk[ki]];
            for i in 0..l.n {
                w.hits.clear();
                f.native_probe(run, v.base, probe_col, lcol[i], &mut w.hits);
                touched += w.hits.len() as u64;
                let knocked = w.hits.iter().any(|h| view_holds(f, run, &v, *h) && (0..rk.len()).all(|k| pr.rfile[k] || super::view_cols::cell(f, run, v.base, *h, v.map[rk[k]]) == l.cols[lk[k]][i]));
                if !knocked {
                    w.keep.push(i as u32);
                }
            }
            gather(pr, l, &w.keep)
        });
        ex.stats.touched[node] += touched;
        return Ok(out);
    }
    let saved = ex.over.take();
    let r = right(ex, f, run, p, pr, l);
    ex.over = saved;
    let r = r?;
    if r.n == 0 {
        return Ok(l.clone());
    }
    ex.stats.touched[node] += r.n as u64;
    Ok(WORK.with(|w| {
        let w = &mut *w.borrow_mut();
        knock(pr, l, &r, w);
        gather(pr, l, &w.keep)
    }))
}

/// The right side: probed by the left's distinct keys (first-seen order) where it is pending, else made.
fn right(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, pr: &Prep, l: &Table) -> R<Rc<Table>> {
    if let Some(i) = pr.rk_at
        && ex.deferred(f, run, pr.right)?
    {
        let mut ks = take_keys();
        let lc = &l.cols[pr.lk[i]];
        if l.n <= 2 * SCAN {
            for k in lc {
                if !ks.contains(k) {
                    ks.push(*k);
                }
            }
        } else {
            WORK.with(|w| {
                let w = &mut *w.borrow_mut();
                // a set grown by a large left is dropped, not cleared (clearing costs its capacity)
                if w.set.capacity() > 8 * l.n && w.set.capacity() > 4096 {
                    w.set = FxHashSet::default();
                }
                w.set.clear();
                ks.extend(lc.iter().copied().filter(|k| w.set.insert(*k)));
            });
        }
        let out = input_probe(ex, f, run, p, pr.right, pr.rk[i], &ks, pr.right_sole);
        give_keys(ks);
        return out;
    }
    if pr.rk_at.is_none() && ex.deferred(f, run, pr.right)? {
        ex.reason = "keyed right side with only file keys";
    }
    ex.eval(f, run, p, pr.right)
}

/// `w.keep`: the left rows whose key no right row holds.
fn knock(pr: &Prep, l: &Table, r: &Table, w: &mut Work) {
    w.keep.clear();
    let (lk, rk) = (&pr.lk, &pr.rk);
    if lk.len() == 1 {
        let (lc, rc) = (&l.cols[lk[0]], &r.cols[rk[0]]);
        if r.n <= SCAN {
            w.keep.extend((0..l.n as u32).filter(|i| !rc.contains(&lc[*i as usize])));
        } else {
            if w.set.capacity() > 8 * r.n && w.set.capacity() > 4096 {
                w.set = FxHashSet::default();
            }
            w.set.clear();
            w.set.extend(rc.iter().copied());
            w.keep.extend((0..l.n as u32).filter(|i| !w.set.contains(&lc[*i as usize])));
        }
        return;
    }
    let lrow = |i: usize, k: usize| l.cols[lk[k]][i];
    let rrow = |j: usize, k: usize| r.cols[rk[k]][j];
    let n = lk.len();
    if r.n <= SCAN {
        w.keep.extend((0..l.n as u32).filter(|i| !(0..r.n).any(|j| (0..n).all(|k| rrow(j, k) == lrow(*i as usize, k)))));
        return;
    }
    let hash = |get: &dyn Fn(usize) -> u64| -> u64 {
        let mut h: u64 = 0;
        for k in 0..n {
            h = (h.rotate_left(5) ^ get(k)).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
        h
    };
    if w.head.capacity() > 8 * r.n && w.head.capacity() > 4096 {
        w.head = FxHashMap::default();
    }
    w.head.clear();
    w.next.clear();
    w.next.resize(r.n, NONE);
    for j in 0..r.n {
        let h = hash(&|k| rrow(j, k));
        if let Some(first) = w.head.get_mut(&h) {
            w.next[j] = *first;
            *first = j as u32;
        } else {
            w.head.insert(h, j as u32);
        }
    }
    for i in 0..l.n {
        let h = hash(&|k| lrow(i, k));
        let mut x = w.head.get(&h).copied().unwrap_or(NONE);
        let mut hit = false;
        while x != NONE {
            if (0..n).all(|k| rrow(x as usize, k) == lrow(i, k)) {
                hit = true;
                break;
            }
            x = w.next[x as usize];
        }
        if !hit {
            w.keep.push(i as u32);
        }
    }
}

/// The left's kept rows: the left itself when all are kept, the prepared empty table when none.
fn gather(pr: &Prep, l: &Rc<Table>, keep: &[u32]) -> Rc<Table> {
    if keep.len() == l.n {
        return l.clone();
    }
    if keep.is_empty() {
        return pr.empty.clone();
    }
    // redo-alloc: the table and its columns from the worker's pools (recycle.rs)
    let mut out = Table::with_rows(l.names.clone(), l.tys.clone(), keep.len());
    for (c, src) in l.cols.iter().enumerate() {
        let mut v = std::mem::take(&mut out.cols[c]);
        v.extend(keep.iter().map(|i| src[*i as usize]));
        out.cols[c] = v;
    }
    out.n = keep.len();
    super::recycle::rc(out)
}
