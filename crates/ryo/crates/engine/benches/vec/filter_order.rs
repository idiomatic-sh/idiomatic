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
        E::Not(a) | E::Null(a, _) | E::Re(a, _) => file_const(a),
        E::Seed(..) => false,
    }
}

fn subst(e: &E, file: &E) -> E {
    let b = |x: &E| Box::new(subst(x, file));
    match e {
        E::Col(_, _, Ty::File) => file.clone(),
        E::List(xs) => E::List(xs.iter().map(|x| subst(x, file)).collect()),
        E::Call(n, xs) => E::Call(n.clone(), xs.iter().map(|x| subst(x, file)).collect()),
        E::Cmp(o, x, y) => E::Cmp(*o, b(x), b(y)),
        E::In(x, y, n) => E::In(b(x), b(y), *n),
        E::And(x, y) => E::And(b(x), b(y)),
        E::Or(x, y) => E::Or(b(x), b(y)),
        E::Not(x) => E::Not(b(x)),
        E::Null(x, n) => E::Null(b(x), *n),
        E::Re(x, r) => E::Re(b(x), r.clone()),
        other => other.clone(),
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct PathList {
    start: u32,
    count: u32,
}

impl PathList {
    pub(super) fn is_empty(self) -> bool { self.count == 0 }
}

#[derive(Default)]
pub(super) struct Paths {
    keys: rustc_hash::FxHashMap<String, u32>,
    exprs: Vec<E>,
    tests: Vec<u32>,
}

impl Paths {
    pub(super) fn checkpoint(&self) -> (usize, usize) { (self.exprs.len(), self.tests.len()) }

    pub(super) fn restore(&mut self, (exprs, tests): (usize, usize)) {
        self.tests.truncate(tests);
        self.exprs.truncate(exprs);
        self.keys.retain(|_, id| (*id as usize) < exprs);
    }

    pub(super) fn bind(&mut self, conj: &[E]) -> PathList {
        let start = self.tests.len();
        for e in conj.iter().filter(|e| file_const(e) && !matches!(e, E::Lit(_))) {
            // Every source's file column denotes the current file here. Keep it
            // distinct from a literal file ID when canonicalizing the expression.
            let expr = subst(e, &E::Col(0, 0, Ty::File));
            let id = *self.keys.entry(format!("{expr:?}")).or_insert_with(|| {
                let i = self.exprs.len() as u32;
                self.exprs.push(expr);
                i
            });
            self.tests.push(id);
        }
        PathList { start: start as u32, count: (self.tests.len() - start) as u32 }
    }
}

/// An unresolved function is an observable diagnostic on a conjunction's first
/// evaluation. Sharing its boolean across conjunctions must still report it there.
#[derive(Default)]
pub(super) struct PathErrors {
    names: rustc_hash::FxHashMap<u32, Box<[String]>>,
    reported: rustc_hash::FxHashSet<(u32, u32)>,
}

pub(super) fn file_fails(run: &mut Run, f: &File<'_>, list: &PathList) -> bool {
    if sw!("VEC_NO_FILECONST") { return false; }
    (list.start..list.start + list.count).any(|at| {
        let id = run.path_tests.tests[at as usize];
        if let Some(fails) = f.path_answers.borrow().get(id) {
            if let Some(errors) = f.path_errors.get() {
                let mut errors = errors.borrow_mut();
                if errors.names.contains_key(&id) && errors.reported.insert((list.start, id)) {
                    run.missing.extend(errors.names[&id].iter().cloned());
                }
            }
            return fails;
        }
        let expr = subst(&run.path_tests.exprs[id as usize], &E::Lit(V::new(Ty::File, u64::from(f.file))));
        let sources: [Src; 0] = [];
        let mut previous = std::mem::take(&mut run.missing);
        let fails = !truthy(eval(run, f, &sources, &[], &expr));
        if !run.missing.is_empty() {
            let mut errors = f.path_errors.get_or_init(Default::default).borrow_mut();
            errors.names.insert(id, run.missing.iter().cloned().collect());
            errors.reported.insert((list.start, id));
            previous.append(&mut run.missing);
        }
        run.missing = previous;
        f.path_answers.borrow_mut().set(id, fails, run.path_tests.exprs.len());
        fails
    })
}
