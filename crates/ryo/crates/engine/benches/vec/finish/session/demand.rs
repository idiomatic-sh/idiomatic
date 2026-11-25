//! A closed equality can get its node keys from a file-local posting. For an
//! axis, every returned "related" node belongs to the axis's declared domain.
use super::super::{
    E, Exec, File, Native, Op, Provider, R, Run, Supplier, Table, Ty, native_of,
};
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
        // The preparation executor has disabled all its views so that File
        // equalities survive. Discover the native base without a view lookup.
        if !native_child(exec, run, source) || exec.plan.emits.iter().any(|e| e.1 == source)
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

/// The nodes of one file a closed reader can need from a source, as an
/// over-approximation built from file-local columns alone.
#[derive(Clone, PartialEq, Eq)]
pub(in super::super::super) enum Nodes {
    /// The node values of a file-local step's column, for rows passing literal tests.
    Column { step: usize, column: usize, tests: Vec<(usize, Vec<u8>)> },
    Children(Box<Nodes>),
    Below(Box<Nodes>),
    Above(Box<Nodes>),
    Union(Vec<Nodes>),
}

fn union(mut parts: Vec<Nodes>) -> Nodes {
    let mut seen: Vec<Nodes> = Vec::new();
    for part in parts.drain(..) {
        match part {
            Nodes::Union(inner) => {
                for each in inner {
                    if !seen.contains(&each) {
                        seen.push(each);
                    }
                }
            }
            other => {
                if !seen.contains(&other) {
                    seen.push(other);
                }
            }
        }
    }
    if seen.len() == 1 { seen.pop().unwrap() } else { Nodes::Union(seen) }
}

impl Nodes {
    /// The file-local steps whose rows this set is read from.
    pub(in super::super::super) fn steps(&self, out: &mut Vec<usize>) {
        match self {
            Self::Column { step, .. } => out.push(*step),
            Self::Children(inner) | Self::Below(inner) | Self::Above(inner) => inner.steps(out),
            Self::Union(parts) => parts.iter().for_each(|part| part.steps(out)),
        }
    }
}

fn ancestor_direction(direction: &str) -> Option<bool> {
    match direction {
        "ancestor" | "nearest-ancestor" | "parent" => Some(true),
        "descendant" | "child" => Some(false),
        _ => None,
    }
}

/// A superset of the node values a closed step's column can hold, read from
/// file-local steps: every output cell is copied or looked up from one of them.
fn origin(
    exec: &mut Exec<'_>,
    run: &mut Run,
    local: &[bool],
    step: usize,
    col: usize,
    depth: usize,
) -> Option<Nodes> {
    if depth == 0 || exec.schema(run, step).ok()?.tys.get(col) != Some(&Ty::Node) {
        return None;
    }
    if local[step] {
        return Some(Nodes::Column { step, column: col, tests: Vec::new() });
    }
    let plan = exec.plan;
    let node = &plan.nodes[step];
    match node.op() {
        "select" => {
            let sel = exec.sel(run, step).ok()?;
            let E::Col(side, c, Ty::Node) = &sel.items.get(col)?.1 else {
                return None;
            };
            let (side, c) = (*side, *c);
            let src = sel.srcs[side];
            if !local[src] {
                return origin(exec, run, local, src, c, depth - 1);
            }
            if native_child(exec, run, src) {
                // A child looked up by an equality on its parent.
                if c != 1 {
                    return None;
                }
                for term in &sel.conj {
                    let E::Cmp(Op::Eq, a, b) = term else { continue };
                    let (E::Col(x, px, Ty::Node), E::Col(y, k, Ty::Node)) = (&**a, &**b) else { continue };
                    let (other, key) = if *x == side && *px == 0 && *y != side {
                        (*y, *k)
                    } else if *y == side && *k == 0 && *x != side {
                        (*x, *px)
                    } else {
                        continue;
                    };
                    let inner = origin(exec, run, local, sel.srcs[other], key, depth - 1)?;
                    return Some(Nodes::Children(Box::new(inner)));
                }
                return None;
            }
            let mut tests = Vec::new();
            for term in &sel.conj {
                let E::Cmp(Op::Eq, a, b) = term else { continue };
                match (&**a, &**b) {
                    (E::Col(x, tc, Ty::Sym), E::Text(l)) | (E::Text(l), E::Col(x, tc, Ty::Sym)) if *x == side => {
                        tests.push((*tc, run.texts[*l as usize].to_vec()));
                    }
                    _ => {}
                }
            }
            Some(Nodes::Column { step: src, column: c, tests })
        }
        "union" => {
            let (left, right) = (exec.arg_rel(step, "left").ok()?, exec.arg_rel(step, "right").ok()?);
            if exec.schema(run, left).ok()?.names != exec.schema(run, right).ok()?.names {
                return None;
            }
            Some(union(vec![
                origin(exec, run, local, left, col, depth - 1)?,
                origin(exec, run, local, right, col, depth - 1)?,
            ]))
        }
        "antijoin" => {
            let left = exec.arg_rel(step, "left").ok()?;
            origin(exec, run, local, left, col, depth - 1)
        }
        "project" => {
            let from = exec.arg_rel(step, "from").ok()?;
            let name = super::super::strs(&node.args()["columns"]).ok()?.nth(col)?;
            let at = super::super::pos(&exec.schema(run, from).ok()?.names, name).ok()?;
            origin(exec, run, local, from, at, depth - 1)
        }
        "extend" => {
            let from = exec.arg_rel(step, "from").ok()?;
            if col >= exec.schema(run, from).ok()?.names.len() {
                return None;
            }
            origin(exec, run, local, from, col, depth - 1)
        }
        "axis" => {
            let from = exec.arg_rel(step, "from").ok()?;
            if col < exec.schema(run, from).ok()?.names.len() {
                return origin(exec, run, local, from, col, depth - 1);
            }
            let relation = exec.arg_rel(step, "relation").ok()?;
            let id = super::super::pos(&exec.schema(run, from).ok()?.names, "id").ok()?;
            let reach = origin(exec, run, local, from, id, depth - 1)?;
            if !native_child(exec, run, relation) {
                return None;
            }
            Some(if ancestor_direction(super::super::s(&node.args()["direction"]).ok()?)? {
                Nodes::Above(Box::new(reach))
            } else {
                Nodes::Below(Box::new(reach))
            })
        }
        _ => None,
    }
}

/// A source read only as the start rows or the domain of axes along the
/// native hierarchy needs only the nodes on the reachable side of the
/// other operand: a row no such node can relate gives the axis no output row.
pub(in super::super::super) fn reach_domains(
    exec: &mut Exec<'_>,
    run: &mut Run,
    local: &[bool],
    reached: &[bool],
    inputs: &BTreeSet<usize>,
) -> BTreeMap<usize, Nodes> {
    let mut found = BTreeMap::new();
    for &source in inputs {
        if exec.plan.emits.iter().any(|e| e.1 == source) {
            continue;
        }
        let mut parts = Vec::new();
        let bounded = exec.plan.consumers(source).iter().copied()
            .filter(|&reader| reached[reader] && !local[reader])
            .all(|reader| {
                let step = &exec.plan.nodes[reader];
                if step.op() != "axis" || step.inputs.iter().filter(|&&i| i == source).count() != 1 {
                    return false;
                }
                let (Ok(from), Ok(domain), Ok(relation)) = (
                    exec.arg_rel(reader, "from"),
                    exec.arg_rel(reader, "domain"),
                    exec.arg_rel(reader, "relation"),
                ) else {
                    return false;
                };
                let Some(ancestors) = super::super::s(&step.args()["direction"]).ok().and_then(ancestor_direction)
                else {
                    return false;
                };
                if !native_child(exec, run, relation) || (from != source && domain != source) {
                    return false;
                }
                let start = from == source;
                let other = if start { domain } else { from };
                let Some(id) = exec.schema(run, other).ok()
                    .and_then(|schema| super::super::pos(&schema.names, "id").ok())
                else {
                    return false;
                };
                let Some(base) = origin(exec, run, local, other, id, 32) else {
                    return false;
                };
                parts.push(if start == ancestors { Nodes::Below(Box::new(base)) } else { Nodes::Above(Box::new(base)) });
                true
            });
        if bounded && !parts.is_empty() {
            found.insert(source, union(parts));
        }
    }
    found
}

/// A native edge relation read only as the hierarchy of descendant walks needs
/// only the edges whose parent is a start node or lies below one.
pub(in super::super::super) fn walk_domains(
    exec: &mut Exec<'_>,
    run: &mut Run,
    local: &[bool],
    reached: &[bool],
    inputs: &BTreeSet<usize>,
) -> BTreeMap<usize, (usize, Nodes)> {
    let mut found = BTreeMap::new();
    for &source in inputs {
        if exec.plan.emits.iter().any(|e| e.1 == source) || !native_child(exec, run, source) {
            continue;
        }
        let mut parts = Vec::new();
        let bounded = exec.plan.consumers(source).iter().copied()
            .filter(|&reader| reached[reader] && !local[reader])
            .all(|reader| {
                let step = &exec.plan.nodes[reader];
                if step.op() != "axis"
                    || step.inputs.iter().filter(|&&i| i == source).count() != 1
                    || exec.arg_rel(reader, "relation") != Ok(source)
                    || !matches!(super::super::s(&step.args()["direction"]), Ok("descendant" | "child"))
                {
                    return false;
                }
                let Ok(from) = exec.arg_rel(reader, "from") else {
                    return false;
                };
                let Some(id) = exec.schema(run, from).ok()
                    .and_then(|schema| super::super::pos(&schema.names, "id").ok())
                else {
                    return false;
                };
                let Some(start) = origin(exec, run, local, from, id, 32) else {
                    return false;
                };
                parts.push(union(vec![start.clone(), Nodes::Below(Box::new(start))]));
                true
            });
        if bounded && !parts.is_empty() {
            // The edges are keyed by their parent column.
            found.insert(source, (0, union(parts)));
        }
    }
    found
}

/// The file's nodes on one side of the given nodes, strictly, ascending.
pub(super) fn reach_nodes(
    run: &Run,
    file: &mut File<'_>,
    from: &[u32],
    below: bool,
) -> Vec<u32> {
    let mut seen = rustc_hash::FxHashSet::default();
    let mut pending: Vec<u32> = from.to_vec();
    let mut children = Vec::new();
    while let Some(node) = pending.pop() {
        if below {
            children.clear();
            file.native_probe(run, Native::Child, 0, u64::from(node), &mut children);
            pending.extend(children.iter().copied().filter(|&child| seen.insert(child)));
        } else if file.has_parent(node) {
            let parent = file.parent[node as usize];
            if seen.insert(parent) {
                pending.push(parent);
            }
        }
    }
    let mut nodes: Vec<_> = seen.into_iter().collect();
    nodes.sort_unstable();
    nodes
}

/// Evaluate a demanded node set in this file: ascending and distinct.
pub(super) fn nodes_of(
    exec: &mut Exec<'_>,
    run: &mut Run,
    file: &mut File<'_>,
    facts: &mut dyn Provider,
    slots: &[usize],
    nodes: &Nodes,
) -> R<Vec<u32>> {
    let mut out: Vec<u32> = match nodes {
        Nodes::Column { step, column, tests } => {
            let table = exec.eval(file, run, facts, slots[*step])?;
            let words: Vec<(usize, u64)> = tests.iter().map(|(c, text)| (*c, u64::from(run.sym(text)))).collect();
            (0..table.n)
                .filter(|&r| words.iter().all(|&(c, w)| table.cols[c][r] == w))
                .map(|r| table.cols[*column][r])
                .filter(|&node| node < file.n as u64)
                .map(|node| node as u32)
                .collect()
        }
        Nodes::Children(inner) => {
            let parents = nodes_of(exec, run, file, facts, slots, inner)?;
            let mut rows = Vec::new();
            for parent in parents {
                file.native_probe(run, Native::Child, 0, u64::from(parent), &mut rows);
            }
            rows
        }
        Nodes::Below(inner) => {
            let from = nodes_of(exec, run, file, facts, slots, inner)?;
            reach_nodes(run, file, &from, true)
        }
        Nodes::Above(inner) => {
            let from = nodes_of(exec, run, file, facts, slots, inner)?;
            reach_nodes(run, file, &from, false)
        }
        Nodes::Union(parts) => {
            let mut all = Vec::new();
            for part in parts {
                all.extend(nodes_of(exec, run, file, facts, slots, part)?);
            }
            all
        }
    };
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

fn native_child(exec: &Exec<'_>, run: &Run, source: usize) -> bool {
    exec.plan.nodes[source].base().is_some_and(|name| {
        matches!(run.supplier(name), Supplier::Provider)
            && native_of(run, name) == Some(Native::Child)
    })
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
    if !native_child(exec, run, source) {
        return Ok(None);
    }
    let domain = exec.eval(file, run, facts, domain)?;
    let id = domain.col("id")?;
    let schema = exec.schema(run, source)?;
    let sample = super::super::operators::enter(source, domain.n);
    let mut pending: Vec<_> = domain.cols[id].iter().copied()
        .filter(|&node| node < file.n as u64)
        .map(|node| node as u32)
        .collect();
    // A provider can lend physical tokens after the AST positions. Follow
    // the same child postings as the native relation, then restore row order.
    let mut seen = rustc_hash::FxHashSet::default();
    let mut children = Vec::new();
    while let Some(parent) = pending.pop() {
        children.clear();
        file.native_probe(run, Native::Child, 0, u64::from(parent), &mut children);
        pending.extend(children.iter().copied().filter(|child| seen.insert(*child)));
    }
    let mut children: Vec<_> = seen.into_iter().collect();
    children.sort_unstable();
    let mut edges = Table::retained(schema.names.clone(), schema.tys.clone(), children.len());
    for child in children {
        edges.cols[0].push(u64::from(file.parent[child as usize]));
        edges.cols[1].push(u64::from(child));
    }
    edges.n = edges.cols[1].len();
    if let Some(sample) = sample {
        super::super::operators::leave(sample, "ancestor-frontier", edges.n, true);
    }
    Ok(Some(Rc::new(edges)))
}

/// A closed equality reads a source only at keys its file-local drivers can
/// hold: the source column carrying the key and the nodes those keys come from.
pub(in super::super::super) fn domains(
    exec: &mut Exec<'_>,
    run: &mut Run,
    local: &[bool],
    inputs: &BTreeSet<usize>,
) -> BTreeMap<usize, (usize, Nodes)> {
    let mut found = BTreeMap::new();
    for &source in inputs {
        if exec.plan.emits.iter().any(|e| e.1 == source) {
            continue;
        }
        let uses = exec.plan.consumers(source);
        let mut binding: Option<usize> = None;
        let mut parts = Vec::new();
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
                Some((col, origin(exec, run, local, other, driver, 32)?))
            });
            let Some((col, nodes)) = key else {
                return false;
            };
            parts.push(nodes);
            *binding.get_or_insert(col) == col
        });
        if bounded && let Some(col) = binding {
            found.insert(source, (col, union(parts)));
        }
    }
    found
}
