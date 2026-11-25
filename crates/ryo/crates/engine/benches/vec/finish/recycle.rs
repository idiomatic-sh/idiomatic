//! THROWAWAY (ryo-fmx3.1 redo-alloc): where the second half's answers get their memory and give it back. A table's
//! column buffers come from the worker's column pool (`POOL`, finish.rs, step 8); here the rest of a table: its list of
//! columns (`take_cols` / `give_cols`) and the shared answer's own allocation (`rc` / `recycle`: an answer no one else
//! holds is kept, emptied, and handed out again as the next answer, so no answer is allocated or freed in a steady run).
//! Nothing here changes what a step computes: only where its table's memory comes from.

use super::Table;
use std::cell::RefCell;
use std::rc::Rc;

/// At most this many emptied answers and column lists are kept (a worker's working set; the rest are freed).
const KEEP: usize = 1 << 14;

thread_local! {
    static COLS: RefCell<Vec<Vec<Vec<u64>>>> = const { RefCell::new(Vec::new()) };
    static RCS: RefCell<Vec<Rc<Table>>> = const { RefCell::new(Vec::new()) };
    static NO_ROWS: Rc<Vec<u32>> = Rc::new(Vec::new());
    static ROW_LISTS: RefCell<(Vec<Rc<Vec<u32>>>, usize)> = const { RefCell::new((Vec::new(), 0)) };
}

/// The kept row lists' room in all (cells): a worker keeps no more than this (4 MB).
const ROW_CELLS: usize = 1 << 20;

/// A shared row list made by `fill` into a kept list's room when there is one.
#[inline]
pub(super) fn rows_with(fill: impl FnOnce(&mut Vec<u32>)) -> Rc<Vec<u32>> {
    let kept = ROW_LISTS.with(|r| {
        let r = &mut *r.borrow_mut();
        let x = r.0.pop();
        if let Some(x) = &x {
            r.1 -= x.capacity();
        }
        x
    });
    let mut rc = kept.unwrap_or_default();
    match Rc::get_mut(&mut rc) {
        Some(v) => {
            v.clear();
            fill(v);
            rc
        }
        None => {
            let mut v = Vec::new();
            fill(&mut v);
            Rc::new(v)
        }
    }
}

/// A row list given back: kept when no one else holds it (and the worker's room allows).
#[inline]
pub(super) fn recycle_rows(mut r: Rc<Vec<u32>>) {
    if let Some(v) = Rc::get_mut(&mut r) {
        let cap = v.capacity();
        if cap == 0 {
            return;
        }
        let _ = ROW_LISTS.try_with(|p| {
            let p = &mut *p.borrow_mut();
            if p.1 + cap <= ROW_CELLS && p.0.len() < KEEP {
                p.1 += cap;
                p.0.push(r);
            }
        });
    }
}

/// The worker's one empty row list (shared: never written).
#[inline]
pub(super) fn no_rows() -> Rc<Vec<u32>> {
    NO_ROWS.with(Rc::clone)
}

/// An empty list of columns with room for `k` (a kept one when there is one).
#[inline]
pub(super) fn take_cols(k: usize) -> Vec<Vec<u64>> {
    let v = COLS.with(|c| c.borrow_mut().pop());
    match v {
        Some(mut v) => {
            v.reserve(k);
            v
        }
        None => Vec::with_capacity(k),
    }
}

/// An emptied list of columns kept for the next table.
#[inline]
pub(super) fn give_cols(v: Vec<Vec<u64>>) {
    if { static S: std::sync::OnceLock<bool> = std::sync::OnceLock::new(); *S.get_or_init(|| crate::finish::off_name("pool")) } || v.capacity() == 0 || v.capacity() > 64 {
        return;
    }
    debug_assert!(v.is_empty());
    let _ = COLS.try_with(|c| {
        let mut c = c.borrow_mut();
        if c.len() < KEEP {
            c.push(v);
        }
    });
}

/// `t` as a shared answer, in a kept allocation when there is one.
#[inline]
pub(super) fn rc(t: Table) -> Rc<Table> {
    let kept = RCS.with(|r| r.borrow_mut().pop());
    match kept {
        Some(mut r) => {
            // a kept answer is held by no one else (`recycle` checked): its emptied table is dropped for `t`
            if let Some(slot) = Rc::get_mut(&mut r) {
                *slot = t;
                r
            } else {
                Rc::new(t)
            }
        }
        None => Rc::new(t),
    }
}

/// An answer given back: kept for the next `rc` when no one else holds it (its columns back in their pools), else only
/// this hold of it dropped.
#[inline]
pub(super) fn recycle(mut r: Rc<Table>) {
    if let Some(t) = Rc::get_mut(&mut r) {
        t.release();
        let _ = RCS.try_with(|p| {
            let mut p = p.borrow_mut();
            if p.len() < KEEP {
                p.push(r);
            }
        });
    }
}
