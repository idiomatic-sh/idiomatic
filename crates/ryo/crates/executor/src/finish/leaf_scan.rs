//! Attribute view materializations to their source plan nodes.
use super::{Exec, File, Native, Run, ViewSpec, kinds_required, step_clock};
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Driver {
    pub calls: u64,
    pub rows: u64,
    pub seconds: f64,
}

thread_local! {
    static TESTS: RefCell<BTreeMap<(usize, bool), u64>> = const { RefCell::new(BTreeMap::new()) };
    static LISTS: RefCell<BTreeMap<(usize, bool), (Driver, u64)>> = const { RefCell::new(BTreeMap::new()) };
}

/// A leaf-only CPU clock for the share of an otherwise unclocked second half.
#[inline]
pub fn table_clock() -> f64 {
    if sw!("VEC_LEAF_CLOCK") {
        super::super::thread_cpu()
    } else {
        step_clock()
    }
}

/// Diagnostic only. Keep the existing evaluator and short-circuit order, recording
/// the number of candidate rows each top-level compiled conjunct actually sees.
pub fn count_tests(
    f: &File<'_>,
    run: &mut Run,
    v: &ViewSpec,
    node: usize,
    materialize: bool,
    rows: &mut Vec<u32>,
) -> u64 {
    let mut n = 0;
    for test in &v.filter {
        if rows.is_empty() {
            break;
        }
        n += rows.len() as u64;
        let one = ViewSpec::new(run, v.base, Vec::new(), vec![test.clone()]);
        super::view_cols::retain(f, run, &one, rows);
    }
    if sw!("VEC_LEAF_TESTS") {
        TESTS.with_borrow_mut(|all| *all.entry((node, materialize)).or_default() += n);
    }
    n
}

/// A cold row-list read outside materialization is timed in its consumer's step.
pub fn row_list(node: usize, materialize: bool, entered: usize, kept: usize, start: f64) {
    let seconds = step_clock() - start;
    LISTS.with_borrow_mut(|all| {
        let (d, scanned) = all.entry((node, materialize)).or_default();
        d.calls += 1;
        d.rows += kept as u64;
        d.seconds += seconds;
        *scanned += entered as u64;
    });
}

impl Exec<'_> {
}


/// Current view_rows entries. Fact column/value lookup follows the conjunct and
/// column order of lit_pick. The child relation presently reads every child.
pub fn fact_entry(
    run: &mut Run,
    i: u16,
    filter: &[super::E],
) -> Option<(usize, String, Vec<u32>)> {
    use super::{E, Ty};
    fn columns(e: &E, out: &mut Vec<usize>) {
        match e {
            E::Col(0, c, _) => out.push(*c),
            E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) | E::In(a, b, _) => {
                columns(a, out);
                columns(b, out);
            }
            _ => {}
        }
    }
    let rel = run.fact_rels[usize::from(i)].0.clone();
    let (_, names, types) = run.catalog(&rel).expect("loaded fact");
    let mut cols = Vec::new();
    for e in filter {
        cols.clear();
        columns(e, &mut cols);
        for &c in &cols {
            if types[c] == Ty::Sym
                && let Some(xs) = kinds_required(e, 0, c)
            {
                return Some((c, names[c].clone(), xs));
            }
        }
    }
    None
}

