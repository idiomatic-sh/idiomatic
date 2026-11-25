//! A closed equality can get its node keys from a file-local posting. For an
//! axis, every returned "related" node belongs to the axis's declared domain.
use super::super::{E, Exec, File, Native, Op, Position, Provider, R, Run, Table, Ty, node_table};
use std::{collections::{BTreeMap, BTreeSet}, rc::Rc};

/// Class numbers describe an equivalence within one invocation. Splitting the
/// invocation preserves comparisons within a file, but can renumber the cells.
/// Follow every use until the class column is dropped.
pub(in super::super::super) fn local_class(
    exec: &mut Exec<'_>,
    run: &mut Run,
    local: &[bool],
    origin: usize,
) -> R<bool> {
    let plan = exec.plan;
    // Class walks use raw node IDs as map keys. Typed catalogue node IDs
    // identify actual nodes, while open provider columns can carry NULL.
    // A null endpoint shared by files would make the walk cross a partition.
    let mut reached = vec![false; origin + 1];
    reached[origin] = true;
    for (node, step) in plan.nodes[..=origin].iter().enumerate().rev() {
        if !reached[node] {
            continue;
        }
        if let Some(name) = step.base()
            && ryo_relations::domain::relations::service::relation_named(
                ryo_relations::domain::labels::types::NameRef(name),
            ).is_none()
        {
            return Ok(false);
        }
        for &source in &step.inputs {
            reached[source] = true;
        }
    }
    let mut classes = vec![Vec::new(); plan.nodes.len()];
    classes[origin].push(exec.schema(run, origin)?.names.len() - 1);
    for (node, step) in plan.nodes.iter().enumerate().skip(origin + 1) {
        if !step.inputs.iter().any(|&input| !classes[input].is_empty()) {
            continue;
        }
        // The same equality matches the same rows if the right side is
        // read only for existence. Counted rewrites have additional readers.
        if !local[node] || step.op() != "select" || step.counted() {
            return Ok(false);
        }
        let select = exec.sel(run, node)?;
        let class_column = |expr: &E| matches!(
            expr,
            E::Col(side, col, _) if classes[select.srcs[*side]].contains(col)
        );
        let reads_class = |expr: &E| {
            let mut pending = vec![expr];
            while let Some(expr) = pending.pop() {
                if class_column(expr) {
                    return true;
                }
                match expr {
                    E::List(items) | E::Call(_, items) => pending.extend(items),
                    E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) | E::In(a, b, _) => {
                        pending.extend([&**a, &**b]);
                    }
                    E::Not(a) | E::TextTest(a, _) => pending.push(a),
                    E::Lit(_) | E::Num(_) | E::Text(_) | E::Col(..) => {}
                }
            }
            false
        };
        for expr in &select.conj {
            if reads_class(expr)
                && !matches!(expr, E::Cmp(Op::Eq, a, b) if class_column(a) && class_column(b))
            {
                return Ok(false);
            }
        }
        let mut carried = Vec::new();
        for (column, (_, expr)) in select.items.iter().enumerate() {
            if class_column(expr) {
                carried.push(column);
            } else if reads_class(expr) {
                return Ok(false);
            }
        }
        classes[node] = carried;
    }
    Ok(plan.emits.iter().all(|emit| classes[emit.1].is_empty()))
}

/// An ancestor axis only reads edges below a node in its output domain.
/// Keep the proof on the frontier, since the same edge can also have a full reader.
pub(in super::super::super) fn ancestor_domains(
    exec: &mut Exec<'_>,
    run: &mut Run,
    local: &[bool],
    reached: &[bool],
    inputs: &BTreeSet<usize>,
) -> BTreeMap<usize, usize> {
    let mut domains = BTreeMap::new();
    for &source in inputs {
        if exec.plan.nodes[source].base().is_none()
            || exec.plan.emits.iter().any(|e| e.1 == source)
            || !exec.view(run, source).ok().flatten().is_some_and(|v| v.base == Native::Child)
        {
            continue;
        }
        let mut common = None;
        let bounded = exec.plan.consumers(source).iter().copied()
            .filter(|&reader| reached[reader] && !local[reader])
            .all(|reader| {
                let step = &exec.plan.nodes[reader];
                if step.op() != "axis"
                    || super::super::s(&step.args()["direction"]) != Ok("ancestor")
                    || exec.arg_rel(reader, "relation") != Ok(source)
                    || step.inputs.iter().filter(|&&i| i == source).count() != 1
                {
                    return false;
                }
                let Ok(domain) = exec.arg_rel(reader, "domain") else {
                    return false;
                };
                local[domain]
                    && exec.schema(run, domain).ok().is_some_and(|schema| {
                        super::super::pos(&schema.names, "id")
                            .is_ok_and(|id| schema.tys[id] == Ty::Node)
                    })
                    && *common.get_or_insert(domain) == domain
            });
        if bounded && let Some(domain) = common {
            domains.insert(source, domain);
        }
    }
    domains
}

pub(super) fn ancestor_edges(
    exec: &mut Exec<'_>,
    run: &mut Run,
    file: &mut File<'_>,
    facts: &mut dyn Provider,
    source: usize,
    domain: usize,
) -> R<Option<Rc<Table>>> {
    // Binding can override the native child edge with a reader-supplied graph.
    if !exec.view(run, source)?.is_some_and(|view| view.base == Native::Child) {
        return Ok(None);
    }
    let domain = exec.eval(file, run, facts, domain)?;
    let id = domain.col("id")?;
    let schema = exec.schema(run, source)?;
    let sample = super::super::operators::enter(source, domain.n);
    let mut starts: Vec<_> = domain.cols[id].iter().copied()
        .filter(|&node| node < file.n as u64)
        .map(|node| node as u32)
        .collect();
    starts.sort_unstable();
    let mut edges = Table::retained(schema.names.clone(), schema.tys.clone(), 0);
    let mut end = 0;
    for start in starts {
        if start < end {
            continue;
        }
        end = node_table::subtree_end_of(file.table, Position(start)).0;
        for child in start + 1..end {
            if file.has_parent(child) {
                edges.cols[0].push(u64::from(file.parent[child as usize]));
                edges.cols[1].push(u64::from(child));
            }
        }
    }
    edges.n = edges.cols[1].len();
    if let Some(sample) = sample {
        super::super::operators::leave(sample, "ancestor-frontier", edges.n, true);
    }
    Ok(Some(Rc::new(edges)))
}

pub(in super::super::super) fn domains(
    exec: &mut Exec<'_>,
    run: &mut Run,
    local: &[bool],
    inputs: &BTreeSet<usize>,
) -> BTreeMap<usize, (usize, usize)> {
    let mut found = BTreeMap::new();
    for &source in inputs {
        if exec.plan.emits.iter().any(|e| e.1 == source) {
            continue;
        }
        let uses = exec.plan.consumers(source);
        let mut binding = None;
        let bounded = uses.iter().copied().filter(|&u| !local[u]).all(|u| {
            if exec.plan.nodes[u].op() != "select" {
                return false;
            }
            let Ok(sel) = exec.sel(run, u) else {
                return false;
            };
            if sel.srcs.len() != 2
                || sel.srcs[0] == sel.srcs[1]
                || !sel
                    .conj
                    .iter()
                    .chain(sel.items.iter().map(|(_, e)| e))
                    .all(super::certificate::total)
            {
                return false;
            }
            let side = usize::from(sel.srcs[1] == source);
            let other = sel.srcs[1 - side];
            let key = sel.conj.iter().find_map(|term| {
                let E::Cmp(Op::Eq, a, b) = term else {
                    return None;
                };
                let (E::Col(x, i, Ty::Node), E::Col(y, j, Ty::Node)) = (&**a, &**b) else {
                    return None;
                };
                if x == y {
                    return None;
                }
                let (col, driver) = if *x == side { (*i, *j) } else { (*j, *i) };
                let schema = exec.schema(run, other).ok()?;
                let domain = if local[other] && schema.names[driver] == "id" {
                    other
                } else if exec.plan.nodes[other].op() == "axis"
                    && driver + 1 == schema.names.len()
                    && schema.names[driver] == "related"
                {
                    exec.arg_rel(other, "domain").ok()?
                } else {
                    return None;
                };
                if !local[domain] {
                    return None;
                }
                Some((col, domain))
            });
            key.is_some_and(|k| *binding.get_or_insert(k) == k)
        });
        if bounded && let Some(key) = binding {
            found.insert(source, key);
        }
    }
    found
}
