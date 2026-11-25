//! Reverse a native equality only when the other input has a known smaller
//! cardinality. Preserve the original pair order for order-sensitive consumers.
use super::{Extent, File, Native, Run, Src, ViewSpec, cold_bucket::Rows, view_probe};
use super::select_kernel::Pushed;
use ryo::languages::{Position, node_table};
use view_probe::Ids;

enum Input<'a> {
    Range(u32),
    Rows(Rows<'a>),
}

impl Input<'_> {
    fn ids(&self) -> Ids<'_> {
        match self {
            Self::Range(n) => Ids::Range(*n),
            Self::Rows(rows) => Ids::View(rows),
        }
    }
}

fn smaller<'a>(f: &File<'a>, view: &ViewSpec, node: usize, limit: usize) -> Option<Input<'a>> {
    if let Some(slot) = f.vrows.slot(&node) {
        return (slot.len() < limit).then(|| Input::Rows(slot.resolve(f.table)));
    }
    if let Some(bucket) = view.cold_bucket.get() {
        let slot = bucket.select(f);
        return (slot.len() < limit).then(|| Input::Rows(slot.resolve(f.table)));
    }
    if !view.filter.is_empty() {
        return None;
    }
    let n = match view.base {
        Native::Tree(_) | Native::Location => f.n,
        Native::Fact(i) => f.fact(usize::from(i))?.len(),
        _ => return None,
    };
    (n < limit).then_some(Input::Range(n as u32))
}

pub(super) fn reverse(
    f: &File<'_>,
    run: &mut Run,
    sources: &[Src],
    driver: usize,
    columns: [usize; 2],
    rows: Ids<'_>,
    pushed: Option<&Pushed>,
    left: &mut Vec<u32>,
    right: &mut Vec<u32>,
) -> Option<(usize, u64)> {
    let other = 1 - driver;
    let (Src::View(old, _), Src::View(new, node)) = (&sources[driver], &sources[other]) else {
        return None;
    };
    // These driver streams contain each position once, in increasing order.
    // More general cached filters may have a literal-posting order of their own.
    if rows.len() == 0 || (!old.filter.is_empty() && old.cold_bucket.get().is_none()) {
        return None;
    }
    let key = old.map[columns[driver]];
    if !view_probe::supports(f, old, key) {
        return None;
    }
    // Evaluating a generic view expression can intern a fresh symbol. Restrict
    // reversal to views whose original probe filter has no such side effects.
    if !new.filter.is_empty() && new.cold_bucket.get().is_none() {
        return None;
    }
    let input = smaller(f, new, *node, rows.len())?;
    let ids = input.ids();
    let batch = view_probe::Driver {
        rows: ids,
        source: other,
        column: columns[other],
    };
    let mut pairs = Vec::new();
    let hits = view_probe::filtered_pairs(f, run, sources, batch, old, key, pushed, |_, new_id, old_id| {
        pairs.push((old_id, new_id));
    })?;
    if (new.base, new.map[columns[other]]) == (Native::Child(Extent::Whole), 0) {
        pairs
            .sort_unstable_by_key(|&(a, b)| (a, node_table::ordinal_of(f.table, Position(b)).0, b));
    } else {
        pairs.sort_unstable();
    }
    for (a, b) in pairs {
        left.push(a);
        right.push(b);
    }
    Some((ids.len(), hits))
}
