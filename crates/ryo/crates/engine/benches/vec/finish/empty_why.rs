//! THROWAWAY (ryo-fmx3.1, redo-union pass 2): `VEC_EMPTY_WHY`: where the whole step runs that return nothing come from.
//! Each run is classed at its entry (what the file already knows) and, when it returned nothing, tallied by its operator
//! and class; the tally is printed after the run.

use super::{E, Exec, File, Res, Run};
use std::cell::RefCell;
use std::collections::BTreeMap;

thread_local! {
    static TALLY: RefCell<BTreeMap<(String, String), (u64, u64)>> = const { RefCell::new(BTreeMap::new()) };
}

fn made_empty(f: &File<'_>, n: usize) -> bool {
    matches!(&f.memo[n], Some(Res::Tab(t)) if t.n == 0)
}

/// The class of a run of `node` at its entry.
pub(super) fn class(ex: &mut Exec<'_>, f: &File<'_>, run: &mut Run, node: usize) -> (String, Option<usize>) {
    let c = class_(ex, f, run, node);
    let first = match ex.plan.nodes[node].op.as_str() {
        "antijoin" | "ordered-match" | "as-of" => ex.arg_rel(node, "left").ok(),
        "closure" | "follow" => ex.arg_rel(node, "edges").ok(),
        "select" | "union" | "base" | "rows" => None,
        _ => ex.arg_rel(node, "from").ok(),
    };
    (c, first)
}

fn class_(ex: &mut Exec<'_>, f: &File<'_>, run: &mut Run, node: usize) -> String {
    let pn = &ex.plan.nodes[node];
    let op = pn.op.as_str();
    if op == "select" {
        let Ok(sp) = ex.sel(run, node) else { return "select plan error".into() };
        if super::filter_order::file_fails(run, f, &sp.conj, &sp.file_tests) {
            return "a per-file test fails".into();
        }
        if let Some(i) = sp.srcs.iter().position(|s| made_empty(f, *s)) {
            return format!("source {} made empty ({})", i, ex.plan.nodes[sp.srcs[i]].op);
        }
        if sp.conj.iter().any(|e| matches!(e, E::Seed(_, k) if f.seeds.get(*k).is_none_or(Vec::is_empty))) {
            return "seed set empty".into();
        }
        return "select: nothing known".into();
    }
    if op == "union" {
        if pn.inputs.iter().all(|i| made_empty(f, *i)) {
            return "all inputs made empty".into();
        }
        return "union: nothing known".into();
    }
    let first = match op {
        "antijoin" | "ordered-match" | "as-of" => "left",
        "closure" | "follow" => "edges",
        "base" | "rows" => return "no input".into(),
        _ => "from",
    };
    match ex.arg_rel(node, first) {
        Ok(i) if made_empty(f, i) => format!("first input made empty ({})", ex.plan.nodes[i].op),
        Ok(i) if f.memo[i].is_none() => format!("first input not made ({})", ex.plan.nodes[i].op),
        _ => "first input made, rows".into(),
    }
}

/// After the run: tallied (runs, and of them returning nothing).
pub(super) fn tally(ex: &Exec<'_>, f: &File<'_>, node: usize, class: (String, Option<usize>), empty: bool) {
    let pn = &ex.plan.nodes[node];
    let (mut class, first) = class;
    if empty && class.ends_with("nothing known") {
        if let Some(i) = pn.inputs.iter().find(|i| made_empty(f, **i)) {
            class = format!("{class}; input {} made empty in the run", ex.plan.nodes[*i].op);
        }
    }
    if empty && class.starts_with("first input not made") {
        if first.is_some_and(|i| made_empty(f, i)) {
            class = format!("{class}; then empty");
        }
    }
    TALLY.with(|t| {
        let mut t = t.borrow_mut();
        let e = t.entry((pn.op.clone(), class)).or_default();
        e.0 += 1;
        e.1 += u64::from(empty);
    });
}

pub(crate) fn print() {
    TALLY.with(|t| {
        let t = t.borrow();
        let mut v: Vec<_> = t.iter().collect();
        v.sort_by_key(|(_, c)| std::cmp::Reverse(c.1));
        for ((op, class), (n, e)) in v {
            println!("EMPTYWHY\t{op}\t{class}\t{n}\t{e}");
        }
    });
}
