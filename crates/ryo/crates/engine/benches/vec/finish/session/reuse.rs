//! Memo entries are scoped to one prepared closed program and one schema group.
use super::super::{PNode, Res, Table};
use std::rc::Rc;

pub(crate) const HOST: u8 = 1;
pub(crate) const INVENTORY: u8 = 2;
pub(crate) const VARIABLE: u8 = 4;
pub(crate) const RESTORED: u8 = 8;

type Key = (usize, u8, [usize; 3]);

#[derive(Default)]
pub(crate) struct Reuse {
    pub(super) enabled: bool,
    rows: rustc_hash::FxHashMap<Key, Rc<Table>>,
    pub(crate) restored: usize,
    pub(crate) empty_emits: usize,
}

impl Reuse {
    pub(crate) fn new(requests: usize) -> Self {
        Self { enabled: requests > 1 && !crate::ablation::off("CLOSED_STEP_MEMO"), ..Self::default() }
    }
    pub(super) fn get(&mut self, node: usize, deps: u8, mut scope: [usize; 3]) -> Option<Rc<Table>> {
        if deps & VARIABLE != 0 { return None; }
        if deps & HOST == 0 { scope[0] = 0; }
        if deps == 0 { scope = [0; 3]; }
        self.rows.get(&(node, deps, scope)).cloned()
    }

    pub(super) fn remember(&mut self, deps: &[u8], scope: [usize; 3], tables: &[Option<Res>]) {
        self.restored += deps.iter().zip(tables).filter(|(d,t)| **d & RESTORED != 0 && t.is_some()).count();
        if self.rows.len() >= 512 { return; }
        for (node, (&deps, table)) in deps.iter().zip(tables).enumerate() {
            if deps & (VARIABLE | RESTORED) != 0 { continue; }
            let Some(Res::Tab(table)) = table else { continue };
            if table.n.saturating_mul(table.cols.len().max(1)) > 128 { continue; }
            let mut scope = scope;
            if deps & HOST == 0 { scope[0] = 0; }
            if deps == 0 { scope = [0; 3]; }
            self.rows.entry((node,deps,scope)).or_insert_with(|| table.clone());
            if self.rows.len() >= 512 { break; }
        }
    }
}

pub(super) fn empty(step: &PNode, zero: &[bool]) -> bool {
    let input = |arg| step.relation_input(arg).is_some_and(|i| zero[i]);
    match step.op() {
        "union" => !step.inputs.is_empty() && step.inputs.iter().all(|&i| zero[i]),
        "antijoin" | "difference" => input("left"),
        "axis" => input("from"),
        "select" | "intersection" => step.inputs.iter().any(|&i| zero[i]),
        "project" | "extend" | "capture" | "words" | "group-count" | "group-digest"
        | "closure" | "cycles" | "rank" | "span-bounds" => {
            step.inputs.len() == 1 && zero[step.inputs[0]]
        }
        _ => false,
    }
}


thread_local! {
    static COUNTS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}

pub(crate) fn posting() { COUNTS.update(|(p,c)| (p+1,c)); }
pub(crate) fn capture() { COUNTS.update(|(p,c)| (p,c+1)); }
pub(crate) fn take_counts() -> (usize, usize) { COUNTS.replace((0,0)) }
