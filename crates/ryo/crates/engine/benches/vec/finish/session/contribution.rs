//! A local projection read after closure can use the provider's ordinal as a join key.
//! Each child edge has exactly one row with that child ID, in node order.
use super::super::{E, Exec, File, Native, Op, Run, Schema, Table, Ty, ViewSpec, view_cols};
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

pub(in super::super::super) struct Projection {
    demand: Option<(usize, Rc<ViewSpec>)>,
    source: usize,
    tree: Rc<ViewSpec>,
    schema: Rc<Schema>,
    columns: Vec<(usize, Native, usize)>,
    rows: Rows,
    share_with_local_reader: bool,
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum Rows {
    // The visible projection preserves the native row ordinal.
    Unfiltered(Native),
    // All consumers require at least one of these kind or text literals.
    Literals {
        kinds: Vec<u32>,
        text_slots: Vec<usize>,
    },
    // The equality links this node ID to the child column of its parent edge.
    WithParent,
}

// Equality is structural within this reader run. Hashing never identifies a
// result by itself. Filtered views can depend on file-local seeds and stay private.
#[derive(Hash, PartialEq, Eq)]
struct Output {
    rows: Rows,
    columns: Vec<(usize, Native, usize)>,
    names: Rc<Vec<String>>,
    types: Vec<u8>,
    mask: Vec<bool>,
}

#[derive(Default)]
pub(in super::super::super) struct Registry(HashMap<Output, usize>);

impl Registry {
    pub(in super::super::super) fn identify(
        &mut self,
        plans: &BTreeMap<usize, Projection>,
        cut: Option<&super::super::partition::Partition>,
    ) -> HashMap<usize, usize> {
        let mut result = HashMap::new();
        if let Some(cut) = cut {
            for (&node, p) in plans {
                if p.demand.is_some() || !p.tree.filter.is_empty() {
                    continue;
                }
                let key = Output {
                    rows: p.rows.clone(),
                    columns: p.columns.clone(),
                    names: p.schema.names.clone(),
                    types: p.schema.tys.iter().map(|t| *t as u8).collect(),
                    mask: cut.columns[&node].clone(),
                };
                let next = self.0.len();
                result.insert(node, *self.0.entry(key).or_insert(next));
            }
        }
        result
    }
}

impl Projection {
    pub(in super::super::super) fn bind(
        exec: &mut Exec<'_>,
        run: &mut Run,
        cut: Option<&super::super::partition::Partition>,
        tests: &BTreeMap<usize, Vec<(usize, u64)>>,
    ) -> BTreeMap<usize, Self> {
        let mut plans = BTreeMap::new();
        if let Some(cut) = cut {
            for (&node, mask) in &cut.columns {
                let local_reader = exec.plan.consumers(node).iter().any(|&u| cut.file_steps[u]);
                if let Some(mut plan) =
                    Self::prepare(exec, run, node, mask, local_reader, tests.get(&node))
                {
                    if let Some(&(c, domain)) = cut.domains.get(&node)
                        && plan
                            .columns
                            .iter()
                            .any(|&(out, base, col)| out == c && base == Native::Child && col == 0)
                        && let Some(view) = exec.view(run, domain).ok().flatten()
                        && view.base == Native::Tree
                        && !view.filter.is_empty()
                        && exec
                            .schema(run, domain)
                            .ok()
                            .and_then(|s| s.names.iter().position(|n| n == "id"))
                            .is_some_and(|id| view.map[id] == 0)
                        && view
                            .filter
                            .iter()
                            .chain(&plan.tree.filter)
                            .all(super::certificate::total)
                    {
                        plan.demand = Some((domain, view));
                        plan.share_with_local_reader = false;
                    }
                    plans.insert(node, plan);
                }
            }
        }
        plans
    }

    pub(in super::super::super) fn needs_domain(&self) -> bool {
        self.demand.is_some()
    }

    fn prepare(
        exec: &mut Exec<'_>,
        run: &mut Run,
        node: usize,
        mask: &[bool],
        local_reader: bool,
        tests: Option<&Vec<(usize, u64)>>,
    ) -> Option<Self> {
        let step = &exec.plan.nodes[node];
        if step.op != "select" || step.semi().is_some() || step.counted() {
            return None;
        }
        let schema = exec.schema(run, node).ok()?;
        let read = |c: usize| mask.get(c) != Some(&false);
        if let Some(tree) = exec.view(run, node).ok()? {
            let ordinal = match tree.base {
                Native::Tree | Native::Location => Some(0),
                Native::Child => Some(1),
                _ => None,
            };
            if tests.is_none()
                && tree.filter.is_empty()
                && ordinal.is_some_and(|ordinal| tree.map.contains(&ordinal))
            {
                let base = tree.base;
                let columns = tree
                    .map
                    .iter()
                    .enumerate()
                    .filter(|(column, _)| read(*column))
                    .map(|(out, &source)| (out, base, source))
                    .collect();
                return Some(Self {
                    demand: None,
                    source: node,
                    tree,
                    schema,
                    columns,
                    rows: Rows::Unfiltered(base),
                    share_with_local_reader: false,
                });
            }
            // Visiting only the surviving rows must not hide an unsupported
            // expression that the original producer would have reported.
            if tree.base != Native::Tree
                || tree.filter.iter().any(|expr| super::scalar(expr).is_err())
            {
                return None;
            }
            let mut kinds = Vec::new();
            let mut words = Vec::new();
            for &(col, symbol) in tests? {
                match tree.map.get(col)? {
                    1 => kinds.push(u32::try_from(symbol).ok()?),
                    2 => words.push(u32::try_from(symbol).ok()?),
                    _ => return None,
                }
            }
            words.sort_unstable();
            words.dedup();
            let text_slots = super::super::cold_text::slots(run, &words);
            let columns = tree
                .map
                .iter()
                .enumerate()
                .filter(|(c, _)| read(*c))
                .map(|(c, &col)| (c, Native::Tree, col))
                .collect();
            return Some(Self {
                demand: None,
                source: node,
                tree,
                schema,
                columns,
                rows: Rows::Literals { kinds, text_slots },
                share_with_local_reader: false,
            });
        }
        if tests.is_some() || step.inputs.len() != 2 {
            return None;
        }
        let plan = exec.sel(run, node).ok()?;
        let views = [
            exec.view(run, plan.srcs[0]).ok()??,
            exec.view(run, plan.srcs[1]).ok()??,
        ];
        let tree_side = views.iter().position(|v| v.base == Native::Tree)?;
        let edge_side = 1 - tree_side;
        let edge = &views[edge_side];
        if edge.base != Native::Child || !edge.filter.is_empty() {
            return None;
        }
        let [E::Cmp(Op::Eq, a, b)] = plan.conj.as_slice() else {
            return None;
        };
        let endpoint = |e: &E| match e {
            E::Col(side, c, Ty::Node) => Some((views[*side].base, views[*side].map[*c])),
            _ => None,
        };
        let keys = [endpoint(a)?, endpoint(b)?];
        if !keys.contains(&(Native::Tree, 0)) || !keys.contains(&(Native::Child, 1)) {
            return None;
        }
        // A projected ordinal also makes the result distinct under projection.
        if !plan
            .items
            .iter()
            .any(|(_, e)| endpoint(e).is_some_and(|id| keys.contains(&id)))
        {
            return None;
        }
        let mut columns = Vec::new();
        for (c, (_, expr)) in plan.items.iter().enumerate() {
            let E::Col(side, col, _) = expr else {
                return None;
            };
            if local_reader || read(c) {
                columns.push((c, views[*side].base, views[*side].map[*col]));
            }
        }
        Some(Self {
            demand: None,
            source: plan.srcs[tree_side],
            tree: views[tree_side].clone(),
            schema,
            columns,
            rows: Rows::WithParent,
            share_with_local_reader: local_reader,
        })
    }

    fn gather(&self, slots: &[usize], file: &mut File<'_>, run: &mut Run) -> Table {
        let rows = match &self.rows {
            Rows::Unfiltered(base) => file.native_rows(run, *base),
            Rows::Literals { kinds, text_slots } => {
                let mut rows = super::super::bucket(file, run, kinds);
                if !text_slots.is_empty() {
                    let prior = rows.len();
                    super::super::cold_text::candidates(file, run, text_slots, &mut rows);
                    if prior != 0 && rows.len() != prior {
                        rows.sort_unstable();
                        rows.dedup();
                    }
                }
                view_cols::retain_original(file, run, &self.tree, &mut rows);
                rows
            }
            Rows::WithParent if self.demand.is_some() => {
                let (domain, view) = self.demand.as_ref().unwrap();
                let parents = super::super::view_rows_for(file, run, view, slots[*domain], true);
                let mut rows = Vec::new();
                for &parent in parents.iter() {
                    file.native_probe(run, Native::Child, 0, u64::from(parent), &mut rows);
                }
                rows.sort_unstable();
                rows.dedup();
                view_cols::retain_original(file, run, &self.tree, &mut rows);
                rows
            }
            Rows::WithParent => {
                let nodes = super::super::view_rows_for(file, run, &self.tree, slots[self.source], true);
                nodes
                    .iter()
                    .copied()
                    .filter(|&r| file.has_parent(r))
                    .collect()
            }
        };
        let mut table = Table::retained(self.schema.names.clone(), self.schema.tys.clone(), 0);
        for &(dst, base, column) in &self.columns {
            table.cols[dst].reserve_exact(rows.len());
            view_cols::gather(file, run, base, column, &rows, &mut table.cols[dst]);
        }
        table.n = rows.len();
        table
    }
}

pub(super) fn gather(
    plans: &BTreeMap<usize, Projection>,
    node: usize,
    slots: &[usize],
    file: &mut File<'_>,
    run: &mut Run,
) -> Option<Rc<Table>> {
    let plan = plans.get(&node)?;
    if plan.share_with_local_reader
        && let Some(super::super::Res::Tab(t)) = &file.memo[slots[node]]
    {
        return Some(t.clone());
    }
    let table = Rc::new(plan.gather(slots, file, run));
    if plan.share_with_local_reader {
        file.remember(slots[node], super::super::Res::Tab(table.clone()));
    }
    Some(table)
}
