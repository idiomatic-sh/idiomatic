//! A local projection read after closure can use the provider's ordinal as a join key.
//! Each child edge has exactly one row with that child ID, in node order.
use super::super::{E, Exec, Extent, File, Native, Op, Provider, R, Run, Schema, Supplier, Table, Ty, ViewSpec, view_cols};
use super::demand::Nodes;
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

/// Where a reader's keys come from: the demanded rows are those keyed by the
/// nodes a file-local origin can hold, or the nodes themselves.
#[derive(Clone)]
enum Keys {
    Column(Nodes),
    Nodes(Nodes),
}

pub(in super::super::super) struct Projection {
    demand: Option<Keys>,
    source: usize,
    tree: Rc<ViewSpec>,
    schema: Rc<Schema>,
    columns: Vec<(usize, Native, usize)>,
    rows: Rows,
    /// The child relation whose edges a demanded read probes, with the extent the catalog states for it.
    edge: Option<Native>,
    share_with_local_reader: bool,
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum Rows {
    // Evaluate once with the usual provider, then share the carried block.
    ProviderProjection,
    // All consumers require at least one of these kind or text literals.
    Literals {
        kinds: Vec<u32>,
        text_slots: Vec<usize>,
    },
    // The equality links this node ID to the child column of its parent edge.
    WithParent,
    // The view's rows at the demanded nodes only: a row is a node.
    Nodes,
    // The native edges whose parent is a demanded node.
    Edges,
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
                let keys = cut
                    .domains
                    .get(&node)
                    .map(|(column, nodes)| (*column, Keys::Column(nodes.clone())))
                    .or_else(|| cut.reach.get(&node).map(|nodes| (usize::MAX, Keys::Nodes(nodes.clone()))));
                if let Some(plan) =
                    Self::prepare(exec, run, node, mask, local_reader, tests.get(&node), keys)
                {
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
        reader_keys: Option<(usize, Keys)>,
    ) -> Option<Self> {
        let step = &exec.plan.nodes[node];
        if step.op() == "base" {
            let (column, keys) = reader_keys?;
            let Keys::Column(_) = keys else {
                return None;
            };
            let tree = exec.view(run, node).ok()??;
            if !tree.base.is_child() || !tree.filter.is_empty() || tree.map.get(column) != Some(&0) {
                return None;
            }
            let columns = tree
                .map
                .iter()
                .enumerate()
                .filter(|(c, _)| mask.get(*c) != Some(&false))
                .map(|(c, &col)| (c, tree.base, col))
                .collect();
            let edge = tree.base;
            return Some(Self {
                demand: Some(keys),
                source: node,
                schema: exec.schema(run, node).ok()?,
                tree,
                columns,
                rows: Rows::Edges,
                edge: Some(edge),
                share_with_local_reader: false,
            });
        }
        if step.op() != "select" || step.semi().is_some() || step.counted() {
            return None;
        }
        let schema = exec.schema(run, node).ok()?;
        let read = |c: usize| mask.get(c) != Some(&false);
        if let Some(tree) = exec.view(run, node).ok()? {
            if tests.is_none()
                && tree.filter.is_empty()
                && let Native::Fact(fact) = tree.base
                && matches!(run.supplier(&run.fact_rels[usize::from(fact)].0), Supplier::Provider)
            {
                let base = tree.base;
                // Keep the full map in the identity. Even columns omitted by the
                // closed reader can affect duplicate elimination in the producer.
                let columns = tree
                    .map
                    .iter()
                    .enumerate()
                    .map(|(out, &source)| (out, base, source))
                    .collect();
                return Some(Self {
                    demand: None,
                    source: node,
                    tree,
                    schema,
                    columns,
                    rows: Rows::ProviderProjection,
                    edge: None,
                    share_with_local_reader: false,
                });
            }
            // Visiting only the surviving rows must not hide an unsupported
            // expression that the original producer would have reported.
            if !tree.base.is_tree()
                || tree.filter.iter().any(|expr| super::scalar(expr).is_err())
            {
                return None;
            }
            let Some(tests) = tests else {
                // A view is read in place by position: with no literal test it is
                // entered only at the nodes the reader can reach.
                let (column, keys) = reader_keys?;
                let key = match keys {
                    Keys::Column(_) => column,
                    Keys::Nodes(_) => super::super::pos(&schema.names, "id").ok()?,
                };
                if tree.map.get(key) != Some(&0) || !tree.filter.iter().all(super::certificate::total) {
                    return None;
                }
                let columns = tree
                    .map
                    .iter()
                    .enumerate()
                    .filter(|(c, _)| read(*c))
                    .map(|(c, &col)| (c, tree.base, col))
                    .collect();
                return Some(Self {
                    demand: Some(keys),
                    source: node,
                    tree,
                    schema,
                    columns,
                    rows: Rows::Nodes,
                    edge: None,
                    share_with_local_reader: false,
                });
            };
            let mut kinds = Vec::new();
            let mut words = Vec::new();
            for &(col, symbol) in tests {
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
                .map(|(c, &col)| (c, tree.base, col))
                .collect();
            return Some(Self {
                demand: None,
                source: node,
                tree,
                schema,
                columns,
                rows: Rows::Literals { kinds, text_slots },
                edge: None,
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
        let tree_side = views.iter().position(|v| v.base.is_tree())?;
        let edge_side = 1 - tree_side;
        let edge = &views[edge_side];
        if !edge.base.is_child() || !edge.filter.is_empty() {
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
        if !keys.iter().any(|(base, column)| base.is_tree() && *column == 0) || !keys.iter().any(|(base, column)| base.is_child() && *column == 1) {
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
        // Only the edges of the demanded parents can meet the other side.
        let demand = reader_keys.and_then(|(column, keys)| {
            let Keys::Column(_) = keys else {
                return None;
            };
            columns
                .iter()
                .any(|&(out, base, col)| out == column && base.is_child() && col == 0)
                .then_some(keys)
        });
        if demand.is_some() && !views[tree_side].filter.iter().all(super::certificate::total) {
            return None;
        }
        let shared = local_reader && demand.is_none();
        Some(Self {
            demand,
            source: plan.srcs[tree_side],
            tree: views[tree_side].clone(),
            schema,
            columns,
            rows: Rows::WithParent,
            edge: Some(edge.base),
            share_with_local_reader: shared,
        })
    }

    fn gather(
        &self,
        slots: &[usize],
        file: &mut File<'_>,
        run: &mut Run,
        keys: Option<&[u32]>,
    ) -> Option<Table> {
        let rows = match &self.rows {
            Rows::ProviderProjection => return None,
            Rows::Literals { kinds, text_slots } => {
                let mut rows = super::super::bucket(file, run, kinds, self.tree.base.extent());
                if !text_slots.is_empty() {
                    let prior = rows.len();
                    super::super::cold_text::candidates(self.tree.base.extent(), file, run, text_slots, &mut rows);
                    if prior != 0 && rows.len() != prior {
                        rows.sort_unstable();
                        rows.dedup();
                    }
                }
                view_cols::retain_original(file, run, &self.tree, &mut rows);
                rows
            }
            Rows::Nodes => {
                let mut rows = keys?.to_vec();
                view_cols::retain_original(file, run, &self.tree, &mut rows);
                rows
            }
            Rows::Edges | Rows::WithParent if self.demand.is_some() => {
                let mut rows = Vec::new();
                for &parent in keys? {
                    file.native_probe(run, self.edge.expect("a read of edges names its relation"), 0, u64::from(parent), &mut rows);
                }
                rows.sort_unstable();
                rows.dedup();
                view_cols::retain_original(file, run, &self.tree, &mut rows);
                rows
            }
            Rows::Edges => return None,
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
        Some(table)
    }
}

/// The nodes a bound projection can be read at, if its reader demands any.
pub(super) fn demanded(
    plans: &BTreeMap<usize, Projection>,
    node: usize,
    slots: &[usize],
    exec: &mut Exec<'_>,
    file: &mut File<'_>,
    run: &mut Run,
    facts: &mut dyn Provider,
) -> R<Option<Vec<u32>>> {
    let Some(keys) = plans.get(&node).and_then(|p| p.demand.as_ref()) else {
        return Ok(None);
    };
    let (Keys::Column(nodes) | Keys::Nodes(nodes)) = keys;
    Ok(Some(super::demand::nodes_of(exec, run, file, facts, slots, nodes)?))
}

pub(super) fn gather(
    plans: &BTreeMap<usize, Projection>,
    node: usize,
    slots: &[usize],
    file: &mut File<'_>,
    run: &mut Run,
    keys: Option<&[u32]>,
) -> Option<Rc<Table>> {
    let plan = plans.get(&node)?;
    if plan.share_with_local_reader
        && let Some(super::super::Res::Tab(t)) = &file.memo[slots[node]]
    {
        return Some(t.clone());
    }
    let table = Rc::new(plan.gather(slots, file, run, keys)?);
    if plan.share_with_local_reader {
        file.remember(slots[node], super::super::Res::Tab(table.clone()));
    }
    Some(table)
}
