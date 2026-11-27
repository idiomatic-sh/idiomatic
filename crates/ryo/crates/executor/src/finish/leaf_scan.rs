//! Attribute view materializations to their source plan nodes.
use super::{Exec, Native, Run, ViewSpec, kinds_required, step_clock};
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Driver {
    pub(crate) calls: u64,
    pub rows: u64,
    pub(crate) seconds: f64,
}

thread_local! {
    static TESTS: RefCell<BTreeMap<(usize, bool), u64>> = const { RefCell::new(BTreeMap::new()) };
    static LISTS: RefCell<BTreeMap<(usize, bool), (Driver, u64)>> = const { RefCell::new(BTreeMap::new()) };
}

/// A leaf-only CPU clock for the share of an otherwise unclocked second half.
#[inline]
pub(crate) fn table_clock() -> f64 {
    {
        step_clock()
    }
}



impl Exec<'_> {
}


/// Current view_rows entries. Fact column/value lookup follows the conjunct and
/// column order of lit_pick. The child relation presently reads every child.
pub(crate) fn fact_entry(
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

