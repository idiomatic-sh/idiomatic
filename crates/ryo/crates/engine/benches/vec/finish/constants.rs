//! Materialized run constants cross the worker boundary once, on first demand.
//! File tables and all subsequently accessed local memo slots keep ordinary Rc ownership.
use super::{Exec, File, NULL, Provider, R, Res, Run, Table, Ty};
use std::{
    rc::Rc,
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Default)]
struct Slot {
    building: Mutex<()>,
    table: OnceLock<Stored>,
}
pub(crate) struct Constants {
    slots: Vec<Option<Box<Slot>>>,
    prefix: usize,
}
pub(super) struct Stored {
    names: Vec<String>,
    types: Vec<Ty>,
    columns: Vec<Vec<u64>>,
    rows: usize,
    spellings: rustc_hash::FxHashMap<u64, Box<[u8]>>,
}
impl Stored {
    pub(super) fn capture(table: &Table, run: &Run, prefix: usize) -> Self {
        let mut spellings = rustc_hash::FxHashMap::default();
        for (kind, col) in table.tys.iter().zip(&table.cols) {
            if *kind == Ty::Sym {
                for &id in col {
                    if id != NULL && id >= prefix as u64 {
                        spellings
                            .entry(id)
                            .or_insert_with(|| run.texts[id as usize].clone());
                    }
                }
            }
        }
        Self {
            names: (*table.names).clone(),
            types: (*table.tys).clone(),
            columns: table.cols.clone(),
            rows: table.n,
            spellings,
        }
    }
    pub(super) fn slice(
        &self,
        run: &mut Run,
        rows: &[u32],
        names: super::Names,
        tys: super::Tys,
    ) -> Table {
        let mut out = Table::with_rows(names, tys, rows.len());
        out.n = rows.len();
        for ((kind, col), dst) in self.types.iter().zip(&self.columns).zip(&mut out.cols) {
            for &r in rows {
                let value = col[r as usize];
                dst.push(
                    if *kind == Ty::Sym
                        && let Some(s) = self.spellings.get(&value)
                    {
                        u64::from(run.sym(s))
                    } else {
                        value
                    },
                );
            }
        }
        out
    }

    fn local(&self, run: &mut Run) -> Rc<Table> {
        let mut cols = self.columns.clone();
        if !self.spellings.is_empty() {
            for (kind, col) in self.types.iter().zip(&mut cols) {
                if *kind == Ty::Sym {
                    for id in col {
                        if let Some(s) = self.spellings.get(id) {
                            *id = u64::from(run.sym(s));
                        }
                    }
                }
            }
        }
        Rc::new(Table {
            names: Rc::new(self.names.clone()),
            tys: Rc::new(self.types.clone()),
            cols,
            n: self.rows,
        })
    }
}
impl Exec<'_> {
    pub(crate) fn share_constants(&mut self, run: &Run) -> Arc<Constants> {
        let prefix = run.texts.len();
        let slots = (0..self.plan.nodes.len())
            .map(|i| {
                self.is_constant(run, i).then(|| {
                    let slot = Box::<Slot>::default();
                    if let Some(t) = &self.run_memo[i] {
                        let _ = slot.table.set(Stored::capture(t, run, prefix));
                    }
                    slot
                })
            })
            .collect();
        let shared = Arc::new(Constants { slots, prefix });
        self.shared_constants = Some(shared.clone());
        shared
    }

    pub(super) fn shared_constant(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn Provider,
        node: usize,
    ) -> R<Res> {
        let shared = self
            .shared_constants
            .as_ref()
            .expect("shared constants")
            .clone();
        let slot = shared.slots[node].as_ref().expect("constant slot");
        if let Some(table) = slot.table.get() {
            return Ok(self.install_constant(node, table, run));
        }
        if !sw!("VEC_NO_KNOWN") && self.known_empty(f, run, p, node) {
            self.stats.known[node] += 1;
            self.skipped(f, node);
            return Ok(Res::Tab(self.empty_rc(run, node)?));
        }
        // Constant dependencies form a DAG. The lock is acquired only to materialize
        // this constant in a run. A file-dependent step never enters this method.
        let _builder = slot.building.lock().expect("constant builder");
        if let Some(table) = slot.table.get() {
            return Ok(self.install_constant(node, table, run));
        }
        let result = self.materialize(f, run, p, node)?;
        if let Some(t) = &self.run_memo[node] {
            let _ = slot.table.set(Stored::capture(t, run, shared.prefix));
        }
        Ok(result)
    }
    fn install_constant(&mut self, node: usize, stored: &Stored, run: &mut Run) -> Res {
        let table = stored.local(run);
        self.run_memo[node] = Some(table.clone());
        Res::Tab(table)
    }
}
impl Constants {
    pub(super) fn contains(&self, node: usize) -> bool {
        self.slots[node].is_some()
    }
}
