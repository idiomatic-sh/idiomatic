//! THROWAWAY (ryo-fmx3.1, redo-union): the antijoin step as a kernel. Prepared once a run per step: the key columns on
//! both sides, whether the right side is a view probed in place on one of its keys (a knock-out per left row, nothing
//! made) or a table (a set of its keys). A call that knocks out no row hands the left on (by key: itself, no copy); one that knocks out all hands
//! on the prepared empty table; else the kept rows are gathered into exact-size columns.

use super::{Exec, File, Provider, R, Res, Run, Table, Ty, view_holds};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct Prep {
    pub(crate) left: usize,
    pub(crate) right: usize,
    pub(crate) lk: Box<[usize]>,
    pub(crate) rk: Box<[usize]>,
    /// The right key columns that hold the file (always equal within a file in the view path).
    pub(crate) rfile: Box<[bool]>,
    /// The right side is a view with a keyed read on this key.
    pub(crate) view_key: Option<usize>,
    pub empty: Rc<Table>,
}

#[derive(Default)]
pub struct Work {
    pub(crate) keep: Vec<u32>,
    pub(crate) hits: Vec<u32>,
    pub(crate) set: FxHashSet<u64>,
    pub(crate) head: FxHashMap<u64, u32>,
    pub(crate) next: Vec<u32>,
}

thread_local! {
    static WORK: RefCell<Work> = RefCell::new(Work::default());
}

/// Right sides this many rows or fewer are scanned, not hashed.
pub(crate) const SCAN: usize = 8;
pub(crate) const NONE: u32 = u32::MAX;

pub(crate) fn prep(ex: &mut Exec<'_>, run: &mut Run, node: usize) -> R<Rc<Prep>> {
    if let Some(pr) = &ex.antijoin_preps[node] {
        return Ok(pr.clone());
    }
    let plan = ex.plan;
    let a = plan.nodes[node].args();
    let (left, right) = (ex.arg_rel(node, "left")?, ex.arg_rel(node, "right")?);
    let (lsc, rsc) = (ex.schema(run, left)?, ex.schema(run, right)?);
    let lk: Box<[usize]> = super::strs(&a["left-on"])?.map(|c| super::pos(&lsc.names, c)).collect::<R<_>>()?;
    let rk: Box<[usize]> = super::strs(&a["right-on"])?.map(|c| super::pos(&rsc.names, c)).collect::<R<_>>()?;
    let rfile = rk.iter().map(|c| rsc.tys[*c] == Ty::File).collect();
    let view_key = match ex.view(run, right)? {
        Some(v) => (0..rk.len()).find(|i| File::probes(v.base, v.map[rk[*i]])),
        None => None,
    };
    let sc = ex.schema(run, node)?;
    let pr = Rc::new(Prep {
        left,
        right,
        lk,
        rk,
        rfile,
        view_key,
        empty: Rc::new(Table::with(sc.names.clone(), sc.tys.clone())),
    });
    ex.antijoin_preps[node] = Some(pr.clone());
    Ok(pr)
}

/// The antijoin made whole (body of the step; its left is not empty: the body returned before).
pub(crate) fn whole(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
    let pr = prep(ex, run, node)?;
    let l = ex.eval(f, run, p, pr.left)?;
    if l.n == 0 {
        return Ok(pr.empty.clone());
    }
    over(ex, f, run, p, &pr, &l)
}

/// redo-whole pass 2 (keyed.rs): the antijoin read by key: its left read by the same keys (its output columns are the
/// left's), then knocked out as a whole run knocks out. None when the left holds no such row; else (left, answer).
pub(crate) fn keyed_left(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64]) -> R<Option<(Rc<Table>, Rc<Table>)>> {
    let pr = prep(ex, run, node)?;
    let l = ex.probe(f, run, p, pr.left, col, keys)?;
    if l.n == 0 {
        return Ok(None);
    }
    let out = over(ex, f, run, p, &pr, &l)?;
    Ok(Some((l, out)))
}

/// The left rows no right row matches on every key.
pub(crate) fn over(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, pr: &Prep, l: &Rc<Table>) -> R<Rc<Table>> {
    if let Some(right)=f.population(pr.right)
        && l.n<right.n
        && let Some(key)=pr.rk.iter().position(|&c|right.can_probe(c))
    {
        return Ok(WORK.with(|work|{
            let mut w=work.borrow_mut();
            w.keep.clear();
            for i in 0..l.n {
                if !right.contains_keys(key,l,i,&pr.lk,&pr.rk) {w.keep.push(i as u32);}
            }
            gather(pr,l,&w.keep)
        }));
    }
    if let Some(ki) = pr.view_key
        && let Res::View(v) = ex.res(f, run, p, pr.right)?
    {
        let (lk, rk) = (&pr.lk, &pr.rk);
        let out = WORK.with(|w| {
            let w = &mut *w.borrow_mut();
            w.keep.clear();
            let probe_col = v.map[rk[ki]];
            let lcol = &l.cols[lk[ki]];
            for i in 0..l.n {
                w.hits.clear();
                f.native_probe(run, v.base, probe_col, lcol[i], &mut w.hits);
                let knocked = w.hits.iter().any(|h| view_holds(f, run, &v, *h) && (0..rk.len()).all(|k| pr.rfile[k] || super::view_cols::cell(f, run, v.base, *h, v.map[rk[k]]) == l.cols[lk[k]][i]));
                if !knocked {
                    w.keep.push(i as u32);
                }
            }
            gather(pr, l, &w.keep)
        });
        return Ok(out);
    }
    // redo-whole pass 2 (keyed.rs): a right side whose one consumer is this antijoin is read by the left's keys (a left
    // row is knocked out only by a right row holding its keys)
    let r = if ex.keyed_ok(f, run, pr.right, pr.rk[0])? {
        let mut ks: Vec<u64> = l.cols[pr.lk[0]].clone();
        ks.sort_unstable();
        ks.dedup();
        ex.probe(f, run, p, pr.right, pr.rk[0], &ks)?
    } else {
        if pr.rk.len() == 1 && pr.rfile[0] && let Some(out) = ex.file_exclusion(f, run, p, pr.right, (&pr.lk, &pr.rk), l, |keep| gather(pr, l, keep))? {
            return Ok(out);
        }
        ex.eval(f, run, p, pr.right)?
    };
    if r.n == 0 {
        return Ok(l.clone());
    }
    Ok(WORK.with(|w| {
        let w = &mut *w.borrow_mut();
        knock(pr, l, &r, w);
        gather(pr, l, &w.keep)
    }))
}

/// `w.keep`: the left rows whose key no right row holds.
pub(crate) fn knock(pr: &Prep, l: &Table, r: &Table, w: &mut Work) {
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
pub(crate) fn gather(pr: &Prep, l: &Rc<Table>, keep: &[u32]) -> Rc<Table> {
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
