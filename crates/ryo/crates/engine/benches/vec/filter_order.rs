//! THROWAWAY (vec 6e-K, item 4): tests on one row set commute, so a test that reads nothing but the file (its path, or no
//! column at all) is decided once a file before any row is read: false ends the step and asks for none of its inputs.
//! The remaining tests keep their order (kind buckets enter first already; row tests short-circuit as written).

use super::{E, File, Op, Run, Ty, eval_with, truthy};
use std::cell::Cell;

/// Whether a test reads no column but file columns: one value for the whole file.
pub(super) fn file_const(e: &E) -> bool {
    match e {
        E::Col(_, _, t) => *t == Ty::File,
        E::Lit(_) | E::Text(_) | E::Num(_) => true,
        E::List(xs) | E::Call(_, xs) => xs.iter().all(file_const),
        E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) => file_const(a) && file_const(b),
        E::In(a, b, _) => file_const(a) && file_const(b),
        E::Not(a) | E::Null(a, _) | E::Re(a, _) => file_const(a),
        E::Seed(..) => false,
    }
}

/// File-only conjuncts point into their immutable select or view. The owner is
/// run-scoped, so a file number uniquely identifies its cached decision.
pub(super) struct FileTests {
    indexes: Box<[usize]>,
    last: Cell<Option<(u32, bool)>>,
}

impl FileTests {
    pub(super) fn prepare(conj: &[E]) -> Option<Box<Self>> {
        if sw!("VEC_NO_FILECONST") { return None; }
        let indexes: Vec<_> = conj.iter().enumerate().filter_map(|(i, term)| {
            // Both file columns denote this file in a file-only predicate.
            let same_file = matches!(term, E::Cmp(Op::Eq, a, b)
                if matches!((&**a, &**b), (E::Col(_, _, Ty::File), E::Col(_, _, Ty::File))));
            (!same_file && !matches!(term, E::Lit(_)) && file_const(term)).then_some(i)
        }).collect();
        if indexes.is_empty() { return None; }
        Some(Box::new(Self { indexes: indexes.into_boxed_slice(), last: Cell::new(None) }))
    }
}

pub(super) fn file_fails(run: &mut Run, f: &File<'_>, conj: &[E], tests: &Option<Box<FileTests>>) -> bool {
    if f.after_last_file { return false; }
    let Some(tests) = tests else { return false; };
    if let Some((file, failed)) = tests.last.get() && file == f.file { return failed; }
    let failed = tests.indexes.iter().any(|&i| !truthy(eval_with::<true>(run, f, &[], &[], &conj[i])));
    tests.last.set(Some((f.file, failed)));
    failed
}
