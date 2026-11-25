//! THROWAWAY (ryo-fmx3.1, redo-union): the union step as a kernel. A tree of unions whose inner unions only it reads is
//! one n-ary union over its leaves (prepared once a run, per root): the leaves' column positions by output column.
//! A call reads each leaf whole, drops the empty answers, and:
//! - none left: the prepared empty table;
//! - one left whose columns are in the output's order and whose rows are distinct: that answer itself (no copy);
//! - else the rows concatenated in input order, each distinct row once (its first place), into exact-size columns.
//! Distinct rows are found with a hash a row and the worker's reused buffers (a scan of the rows kept for a few rows, a
//! flat map of hash to row chain otherwise); a hash hit is compared row to row.

use super::{Exec, File, Names, Provider, R, Run, Table, Tys};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

struct Leaf {
    node: usize,
    /// The leaf's column of each output column.
    map: Box<[usize]>,
    /// The map is the identity over the leaf's columns.
    ident: bool,
}

pub(super) struct Prep {
    leaves: Box<[Leaf]>,
    k: usize,
    names: Names,
    tys: Tys,
    empty: Rc<Table>,
}

#[derive(Default)]
struct Work {
    /// The rows kept: (answer, row).
    kept: Vec<(u32, u32)>,
    /// Their hashes (small path), or the next row of the same hash (map path).
    hs: Vec<u64>,
    next: Vec<u32>,
    head: FxHashMap<u64, u32>,
}

thread_local! {
    /// Lists of leaf answers, taken and given back around a call (a leaf may recurse into another union).
    static GOT: RefCell<Vec<Vec<(Rc<Table>, u32)>>> = const { RefCell::new(Vec::new()) };
    static WORK: RefCell<Work> = RefCell::new(Work::default());
}

/// Rows this many or fewer are made distinct by a scan of the rows kept.
const SCAN: usize = 16;
const NONE: u32 = u32::MAX;

pub(super) fn prep(ex: &mut Exec<'_>, run: &mut Run, node: usize) -> R<Rc<Prep>> {
    if let Some(pr) = &ex.union_preps[node] {
        return Ok(pr.clone());
    }
    // the tree: inner unions read by this tree alone (and not emitted) are flattened
    let plan = ex.plan;
    let mut tree = vec![node];
    let mut todo = vec![node];
    while let Some(u) = todo.pop() {
        for side in ["left", "right"] {
            let x = ex.arg_rel(u, side)?;
            if plan.nodes[x].op == "union" && ex.plan.consumers(x) == [u] && !ex.emitted.contains(&x) && !tree.contains(&x) {
                tree.push(x);
                todo.push(x);
            }
        }
    }
    // the leaves depth first, left first: the old nested order of rows
    let mut ordered = Vec::new();
    order(ex, node, &tree, &mut ordered)?;
    let sc = ex.schema(run, node)?;
    let mut ls = Vec::with_capacity(ordered.len());
    for &x in &ordered {
        let lsc = ex.schema(run, x)?;
        let map: Box<[usize]> = sc.names.iter().map(|c| super::pos(&lsc.names, c)).collect::<R<_>>()?;
        let ident = map.len() == lsc.names.len() && map.iter().enumerate().all(|(i, c)| i == *c);
        ls.push(Leaf { node: x, map, ident });
    }
    let pr = Rc::new(Prep { leaves: ls.into(), k: sc.names.len(), names: sc.names.clone(), tys: sc.tys.clone(), empty: Rc::new(Table::with(sc.names.clone(), sc.tys.clone())) });
    ex.union_preps[node] = Some(pr.clone());
    Ok(pr)
}

fn order(ex: &Exec<'_>, u: usize, tree: &[usize], out: &mut Vec<usize>) -> R<()> {
    for side in ["left", "right"] {
        let x = ex.arg_rel(u, side)?;
        if tree.contains(&x) && x != u {
            order(ex, x, tree, out)?;
        } else {
            out.push(x);
        }
    }
    Ok(())
}

/// The union made whole (its leaves made whole). pass 2: handed on shared, a single answer passed through uncopied.
pub(super) fn whole(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
    let pr = prep(ex, run, node)?;
    let mut got = take_got();
    for (i, lf) in pr.leaves.iter().enumerate() {
        let t = ex.eval(f, run, p, lf.node)?;
        if t.n > 0 {
            got.push((t, i as u32));
        }
    }
    let out = combine(&pr, &got);
    give_got(got);
    Ok(out)
}

/// redo-whole pass 2 (keyed.rs): the union read by key: each leaf read by the same keys on its own column, combined as
/// a whole run combines; (answer, whether it is a leaf's answer handed on).
pub(super) fn keyed(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64]) -> R<(Rc<Table>, bool)> {
    let pr = prep(ex, run, node)?;
    let mut got = take_got();
    for (i, lf) in pr.leaves.iter().enumerate() {
        let t = ex.probe(f, run, p, lf.node, lf.map[col], keys)?;
        if t.n > 0 {
            got.push((t, i as u32));
        }
    }
    let out = combine(&pr, &got);
    let handed = out.n == 0 || got.iter().any(|g| Rc::ptr_eq(&g.0, &out));
    give_got(got);
    Ok((out, handed))
}

fn take_got() -> Vec<(Rc<Table>, u32)> {
    GOT.with(|k| k.borrow_mut().pop()).unwrap_or_default()
}

fn give_got(mut v: Vec<(Rc<Table>, u32)>) {
    // redo-alloc: the leaves' answers given back (recycle.rs)
    for (t, _) in v.drain(..) {
        super::recycle::recycle(t);
    }
    GOT.with(|k| k.borrow_mut().push(v));
}

fn combine(pr: &Prep, got: &[(Rc<Table>, u32)]) -> Rc<Table> {
    if got.is_empty() {
        return pr.empty.clone();
    }
    let k = pr.k;
    let col = |i: usize, c: usize| -> &[u64] {
        let (t, li) = &got[i];
        &t.cols[pr.leaves[*li as usize].map[c]]
    };
    let total: usize = got.iter().map(|g| g.0.n).sum();
    WORK.with(|w| {
        let w = &mut *w.borrow_mut();
        w.kept.clear();
        let rh = |i: usize, r: usize| -> u64 {
            if k == 1 {
                return col(i, 0)[r];
            }
            let mut h: u64 = 0;
            for c in 0..k {
                h = (h.rotate_left(5) ^ col(i, c)[r]).wrapping_mul(0x517c_c1b7_2722_0a95);
            }
            h
        };
        let same = |a: (u32, u32), i: usize, r: usize| -> bool { k == 1 || (0..k).all(|c| col(a.0 as usize, c)[a.1 as usize] == col(i, c)[r]) };
        if total <= SCAN {
            w.hs.clear();
            for (i, g) in got.iter().enumerate() {
                for r in 0..g.0.n {
                    let h = rh(i, r);
                    let dup = w.hs.iter().zip(&w.kept).any(|(hh, a)| *hh == h && same(*a, i, r));
                    if !dup {
                        w.hs.push(h);
                        w.kept.push((i as u32, r as u32));
                    }
                }
            }
        } else {
            if w.head.capacity() > 8 * total && w.head.capacity() > 4096 {
                w.head = FxHashMap::default();
            }
            w.head.clear();
            w.head.reserve(total);
            w.next.clear();
            for (i, g) in got.iter().enumerate() {
                for r in 0..g.0.n {
                    let h = rh(i, r);
                    let at = w.kept.len() as u32;
                    let first = *w.head.entry(h).or_insert(at);
                    if first != at {
                        let mut x = first;
                        let mut dup = false;
                        while x != NONE {
                            if same(w.kept[x as usize], i, r) {
                                dup = true;
                                break;
                            }
                            x = w.next[x as usize];
                        }
                        if dup {
                            continue;
                        }
                        // a new row of a hash seen: chained after the first
                        let after = w.next[first as usize];
                        w.next[first as usize] = at;
                        w.next.push(after);
                    } else {
                        w.next.push(NONE);
                    }
                    w.kept.push((i as u32, r as u32));
                }
            }
        }
        let n = w.kept.len();
        if n == total && got.len() == 1 && pr.leaves[got[0].1 as usize].ident {
            return got[0].0.clone();
        }
        // redo-alloc: the answer's table and columns from the worker's pools (recycle.rs), not made new
        let mut out = Table::with_rows(pr.names.clone(), pr.tys.clone(), n);
        for c in 0..k {
            let mut v = std::mem::take(&mut out.cols[c]);
            v.reserve(n);
            if n == total {
                for i in 0..got.len() {
                    v.extend_from_slice(col(i, c));
                }
            } else {
                v.extend(w.kept.iter().map(|(i, r)| col(*i as usize, c)[*r as usize]));
            }
            out.cols[c] = v;
        }
        out.n = n;
        super::recycle::rc(out)
    })
}
