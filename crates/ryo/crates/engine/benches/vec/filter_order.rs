//! THROWAWAY (vec 6e-K, item 4): tests on one row set commute, so a test that reads nothing but the file (its path, or no
//! column at all) is decided once a file before any row is read: false ends the step and asks for none of its inputs.
//! The remaining tests keep their order (kind buckets enter first already; row tests short-circuit as written).

use super::{E, File, Run, Src, Ty, V, eval, truthy};

/// Whether a test reads no column but file columns: one value for the whole file.
pub(super) fn file_const(e: &E) -> bool {
    match e {
        E::Col(_, _, t) => *t == Ty::File,
        E::Lit(_) | E::Text(_) | E::Num(_) => true,
        E::List(xs) | E::Call(_, xs) => xs.iter().all(file_const),
        E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) => file_const(a) && file_const(b),
        E::In(a, b, _) => file_const(a) && file_const(b),
        E::Not(a) | E::Re(a, _) | E::Glob(a, _) => file_const(a),
    }
}

fn subst(e: &E, file: u64) -> E {
    let b = |x: &E| Box::new(subst(x, file));
    match e {
        E::Col(_, _, Ty::File) => E::Lit(V::new(Ty::File, file)),
        E::List(xs) => E::List(xs.iter().map(|x| subst(x, file)).collect()),
        E::Call(n, xs) => E::Call(n.clone(), xs.iter().map(|x| subst(x, file)).collect()),
        E::Cmp(o, x, y) => E::Cmp(*o, b(x), b(y)),
        E::In(x, y, n) => E::In(b(x), b(y), *n),
        E::And(x, y) => E::And(b(x), b(y)),
        E::Or(x, y) => E::Or(b(x), b(y)),
        E::Not(x) => E::Not(b(x)),
        E::Glob(x, g) => E::Glob(b(x), g.clone()),
        E::Re(x, r) => E::Re(b(x), r.clone()),
        other => other.clone(),
    }
}

/// Whether some per-file test of `conj` fails in this file (the step is then empty).
pub(super) fn file_fails(run: &mut Run, f: &File<'_>, conj: &[E]) -> bool {
    if f.after_last_file || sw!("VEC_NO_FILECONST") {
        return false;
    }
    // redo-alloc: decided once a file a test list (the plan's lists stay in place while it runs; a later ask in the same
    // file reads the answer, no substituted copy of the test made again)
    thread_local! {
        static SEEN: std::cell::RefCell<(u32, rustc_hash::FxHashMap<(usize, usize), bool>)> = std::cell::RefCell::new((u32::MAX, rustc_hash::FxHashMap::default()));
    }
    let key = (conj.as_ptr() as usize, conj.len());
    if let Some(v) = SEEN.with(|s| {
        let s = s.borrow();
        if s.0 == f.file { s.1.get(&key).copied() } else { None }
    }) {
        return v;
    }
    let srcs: [Src; 0] = [];
    let v = conj.iter().filter(|c| file_const(c) && !matches!(c, E::Lit(_))).any(|c| !truthy(eval(run, f, &srcs, &[], &subst(c, u64::from(f.file)))));
    SEEN.with(|s| {
        let mut s = s.borrow_mut();
        if s.0 != f.file {
            s.0 = f.file;
            s.1.clear();
        }
        s.1.insert(key, v);
    });
    v
}
