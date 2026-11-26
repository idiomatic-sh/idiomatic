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
        E::Not(a) | E::TextTest(a, _) => file_const(a),
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
        E::TextTest(x, r) => E::TextTest(b(x), r.clone()),
        other => other.clone(),
    }
}

/// Which list of a plan node holds the per-file tests: a select's, or a view's filter.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum List {
    Select,
    View,
}

/// Whether some per-file test of `conj` fails in this file (the step is then empty).
pub(super) fn file_fails(run: &mut Run, f: &File<'_>, node: usize, list: List, conj: &[E]) -> bool {
    if f.after_last_file || sw!("VEC_NO_FILECONST") {
        return false;
    }
    // redo-alloc: decided once a file a test list (the plan's lists stay in place while it runs; a later ask in the same
    // file reads the answer, no substituted copy of the test made again). The answer is kept in the file it was decided for,
    // by the plan node the list belongs to: an index of the plan this file is run against, which no other list takes.
    let key = (node, list);
    if let Some(v) = f.file_tests.borrow().get(&key).copied() {
        return v;
    }
    let srcs: [Src; 0] = [];
    let v = conj.iter().filter(|c| file_const(c) && !matches!(c, E::Lit(_))).any(|c| !truthy(eval(run, f, &srcs, &[], &subst(c, u64::from(f.file)))));
    f.file_tests.borrow_mut().insert(key, v);
    v
}

#[cfg(test)]
mod tests {
    use super::super::{FileVersion, Op};
    use super::*;
    use crate::languages::{Count, FieldCode, KindBits, NodeTable, Offset, node_table};

    /// A list of per-file tests lives at a place in the plan of one visit, and the next visit's plan (or another version of the
    /// same path, with the same file number) can have another list at the same place and of the same length: what was decided
    /// for one file is never read by another.
    #[test]
    fn a_file_reads_only_the_tests_decided_for_it() {
        let mut table = NodeTable::default();
        let mut writer = node_table::builder(&mut table, Count(1));
        node_table::enter_node(&mut writer, KindBits(0), FieldCode(0), Offset(0));
        node_table::leave_node(&mut writer, Offset(0));
        let version = FileVersion::new(node_table::finished(&mut writer), b"", 0, None);
        let mut run = Run::default();
        let number = |n: i64| Box::new(E::Num(n));
        let mut conj = vec![E::Cmp(Op::Eq, number(1), number(1))];
        {
            let file = File::new(&version, 0);
            assert!(!file_fails(&mut run, &file, 0, List::Select, &conj), "a test that holds does not fail the file");
        }
        // The same place and length, another test.
        conj[0] = E::Cmp(Op::Eq, number(1), number(2));
        let file = File::new(&version, 0);
        assert!(file_fails(&mut run, &file, 0, List::Select, &conj), "a test that does not hold fails the file, whatever the last file decided");
    }
}
