//! Locate the outputs of file-local steps consumed after admission closes.
//! Every node number addresses the same prepared executor plan in both phases.
use super::{E, Exec, Op, R, Run, Ty};
use std::collections::{BTreeMap, BTreeSet};

/// Reclassify a prepared program after an independent project answer becomes
/// available by File. Every emitted branch must then be file-local.
pub fn with_file_slice(
    exec: &mut Exec<'_>,
    run: &mut Run,
    relations: &[&str],
    wide: impl Fn(&str) -> bool,
    sliced: impl Fn(&str) -> bool,
) -> R<Option<Vec<bool>>> {
    let exact = local_steps(exec, run, |name| !relations.contains(&name) && wide(name), sliced, |ty| {
        matches!(ty, Ty::Node | Ty::File)
    })?;
    if !exec.plan.emits.iter().all(|emit| exact[emit.1] && emit.2 == "file") {
        return Ok(None);
    }
    // Rank is an invocation-wide order. A File key elsewhere in the graph
    // cannot make its ordinal invariant under slicing.
    let mut reads_slice = Vec::with_capacity(exact.len());
    for step in &exec.plan.nodes {
        let reads = step.base().is_some_and(|name| relations.contains(&name))
            || step.inputs.iter().any(|&input| reads_slice[input]);
        if reads && step.op() == "rank" {
            return Ok(None);
        }
        reads_slice.push(reads);
    }
    Ok(Some(exact))
}

#[derive(Clone, Default)]
pub struct Partition {
    pub domains: BTreeMap<usize, (usize, super::session::demand::Nodes)>,
    pub ancestor_domains: BTreeMap<usize, usize>,
    pub reach: BTreeMap<usize, super::session::demand::Nodes>,
    pub inputs: Vec<usize>,
    pub columns: BTreeMap<usize, Vec<bool>>,
    pub filters: BTreeMap<usize, Vec<(usize, Vec<u8>)>>,
    pub local_emit: bool,
    pub file_steps: Vec<bool>,
}

/// A multidirectional join stays local only if its equality edges connect every
/// source with file-local keys. Closed sources without file-domain columns can
/// be used as binding constants when a file-local input drives the step.
///
/// A caller table with File cells is read in every file through its own rows.
/// It is file-local only while each of its readers binds it by File to a
/// file-local input, so no row of another file can reach a result.
pub fn file_steps(
    exec: &mut Exec<'_>,
    run: &mut Run,
    wide: impl Fn(&str) -> bool,
    sliced: impl Fn(&str) -> bool,
) -> R<Vec<bool>> {
    local_steps(exec, run, wide, sliced, |ty| matches!(ty, Ty::Node | Ty::File | Ty::Span))
}

pub fn local_steps(
    exec: &mut Exec<'_>,
    run: &mut Run,
    wide: impl Fn(&str) -> bool,
    sliced: impl Fn(&str) -> bool,
    anchor: impl Fn(Ty) -> bool,
) -> R<Vec<bool>> {
    let mut excluded = BTreeSet::new();
    loop {
        let local = file_steps_by(
            exec,
            run,
            &wide,
            &anchor,
            &|at, name| sliced(name) && !excluded.contains(&at),
        )?;
        let mut loose = Vec::new();
        for at in 0..local.len() {
            if local[at]
                && let Some(name) = exec.plan.nodes[at].base()
                && wide(name)
                && sliced(name)
                && !slice_readers_bind_file(exec, run, at, &local)?
            {
                loose.push(at);
            }
        }
        if loose.is_empty() {
            return Ok(local);
        }
        excluded.extend(loose);
    }
}

/// Every reader of a caller table keeps it to the rows of its own file: an antijoin
/// that removes rows from a file-local side, or a join through an equality of File cells.
pub fn slice_readers_bind_file(
    exec: &mut Exec<'_>,
    run: &mut Run,
    base: usize,
    local: &[bool],
) -> R<bool> {
    let plan = exec.plan;
    if plan.emits.iter().any(|emit| emit.1 == base) {
        return Ok(false);
    }
    for &reader in plan.consumers(base) {
        if !local[reader] {
            return Ok(false);
        }
        let step = &plan.nodes[reader];
        match step.op() {
            "antijoin" => {
                if exec.arg_rel(reader, "right") != Ok(base)
                    || exec.arg_rel(reader, "left") == Ok(base)
                {
                    return Ok(false);
                }
                let schema = exec.schema(run, base)?;
                let mut keyed = false;
                for name in super::strs(&step.args()["right-on"])? {
                    keyed |= schema.tys[super::pos(&schema.names, name)?] == Ty::File;
                }
                if !keyed {
                    return Ok(false);
                }
            }
            "select" => {
                let select = exec.sel(run, reader)?;
                let bound = |alias: usize| {
                    select.conj.iter().any(|term| match term {
                        E::Cmp(Op::Eq, a, b) => matches!(
                            (&**a, &**b),
                            (E::Col(x, _, Ty::File), E::Col(y, _, Ty::File))
                                if x != y && (*x == alias || *y == alias)
                        ),
                        _ => false,
                    })
                };
                if select.srcs.len() < 2
                    || !select.srcs.iter().enumerate().all(|(alias, &src)| src != base || bound(alias))
                {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}

pub fn file_steps_by(
    exec: &mut Exec<'_>,
    run: &mut Run,
    wide: &impl Fn(&str) -> bool,
    anchor: &impl Fn(Ty) -> bool,
    sliced: &impl Fn(usize, &str) -> bool,
) -> R<Vec<bool>> {
    let classes: Vec<_> = exec.plan.nodes.iter().enumerate()
        .filter_map(|(i, step)| (step.op() == "subtree-class").then_some(i))
        .take(2)
        .collect();
    if let [class] = classes.as_slice() {
        let local = classify(exec, run, wide, anchor, sliced, Some(*class))?;
        if local[*class] && super::session::demand::local_class(exec, run, &local, *class)? {
            return Ok(local);
        }
    }
    classify(exec, run, wide, anchor, sliced, None)
}

pub fn classify(
    exec: &mut Exec<'_>,
    run: &mut Run,
    wide: &impl Fn(&str) -> bool,
    anchor: &impl Fn(Ty) -> bool,
    sliced: &impl Fn(usize, &str) -> bool,
    local_class: Option<usize>,
) -> R<Vec<bool>> {
    exec.closed_inputs();
    let plan = exec.plan;
    let mut local = Vec::with_capacity(plan.nodes.len());
    // Source-independent literal relations can be read in a file join or after
    // the last file. Emitting the literal itself still requires one closed answer.
    let mut literal = Vec::with_capacity(plan.nodes.len());
    for (i, step) in plan.nodes.iter().enumerate() {
        // Use the schema that evaluation itself will bind. A request with no admitted
        // files must still refuse an unknown relation or a malformed projection.
        exec.schema(run, i)?;
        let keys_hold = |ex: &mut Exec<'_>, run: &mut Run, from: &str, keys: &str| -> R<bool> {
            let source = ex.arg_rel(i, from)?;
            let s = ex.schema(run, source)?;
            for name in super::strs(&step.args()[keys])? {
                if anchor(s.tys[super::pos(&s.names, name)?]) {
                    return Ok(true);
                }
            }
            Ok(false)
        };
        let op = step.op();
        let constant_base = step.base().is_some_and(|name| {
            matches!(run.supplier(name), super::Supplier::Host)
                && run.catalog(name).is_some_and(|(_, _, types)| {
                    types.iter().all(|&ty| !anchor(ty))
                })
        });
        let constant = op == "rows" || constant_base || (!step.inputs.is_empty()
            && step.inputs.iter().all(|&source| literal[source])
            && matches!(op, "select" | "project" | "extend" | "union" | "difference" | "intersection"));
        literal.push(constant);
        let local_inputs = step.inputs.iter().all(|&source| local[source]);
        let with_literals = match op {
            "select" => step.inputs.iter().any(|&source| local[source])
                && step.inputs.iter().all(|&source| local[source] || literal[source]),
            "antijoin" => local[exec.arg_rel(i, "left")?] && literal[exec.arg_rel(i, "right")?],
            _ => false,
        };
        let here = if constant || !(local_inputs || with_literals) {
            false
        } else {
            match step.op() {
                "base" => {
                    let name = step.base().ok_or("base relation")?;
                    !wide(name) || sliced(i, name)
                }
                "rows" => false,
                "select" => {
                    if step.inputs.len() < 2 {
                        true
                    } else {
                        // The schema above already bound this select with closed inputs,
                        // preserving file equalities in its flattened conjunction.
                        let select = exec.sel(run, i)?;
                        let mut components: Vec<_> = (0..step.inputs.len()).collect();
                        for term in &select.conj {
                            let E::Cmp(Op::Eq, a, b) = term else { continue };
                            let (E::Col(left, _, lt), E::Col(right, _, rt)) = (&**a, &**b) else {
                                continue;
                            };
                            if lt != rt || !anchor(*lt) {
                                continue;
                            }
                            let (a, b) = (components[*left], components[*right]);
                            for c in &mut components {
                                if *c == b {
                                    *c = a;
                                }
                            }
                        }
                        let mut file_components = step.inputs.iter().zip(components)
                            .filter_map(|(&input, component)| local[input].then_some(component));
                        file_components.next().is_some_and(|root| file_components.all(|other| other == root))
                    }
                }
                "project" => {
                    let src = exec.arg_rel(i, "from")?;
                    exec.schema(run, i)?.tys.iter().any(|&t| anchor(t))
                        || !exec.schema(run, src)?.tys.iter().any(|&t| anchor(t))
                }
                "union" | "intersection" | "difference" => {
                    let mut held = true;
                    for &source in &step.inputs {
                        held &= exec.schema(run, source)?.tys.iter().any(|&t| anchor(t));
                    }
                    held
                }
                "antijoin" if with_literals => true,
                "antijoin" | "as-of" | "ordered-match" => {
                    keys_hold(exec, run, "left", "left-on")?
                        && keys_hold(exec, run, "right", "right-on")?
                }
                "order" | "group-count" | "group-digest" | "group-min" | "group-max" | "group-ordinal" => {
                    keys_hold(exec, run, "from", "keys")?
                }
                "rank" | "extend" | "capture" | "span-bounds" | "span-between"
                | "source-matches" | "render-syntax" | "fits-rendered" | "words" | "replace" | "prefixes" => {
                    true
                }
                "closure" | "cycles" | "follow" => {
                    let source = exec.arg_rel(i, "edges")?;
                    let edges = exec.schema(run, source)?;
                    let source = super::pos(&edges.names, super::s(&step.args()["source"])?)?;
                    anchor(edges.tys[source])
                        || (step.op() != "cycles" && keys_hold(exec, run, "edges", "keys")?)
                }
                "ordered-dfs" | "worklist" => {
                    // The partition key or the graph vertex must identify the
                    // source on both sides. All operands, including the lattice,
                    // have already been checked for file-local scope above.
                    let mut held = true;
                    for (input, vertex) in [("from", "root"), ("edges", "source")] {
                        let schema = exec.schema(run, exec.arg_rel(i, input)?)?;
                        let vertex = super::pos(&schema.names, super::s(&step.args()[vertex])?)?;
                        held &= anchor(schema.tys[vertex]) || keys_hold(exec, run, input, "keys")?;
                    }
                    held
                }
                "axis" => true,
                "subtree-class" if local_class == Some(i) => {
                    let id = super::s(&step.args()["id"])?;
                    let mut node_keys = true;
                    for input in ["from", "labels", "leaves", "relation"] {
                        let Ok(source) = exec.arg_rel(i, input) else {
                            node_keys &= input == "leaves";
                            continue;
                        };
                        let sc = exec.schema(run, source)?;
                        if input == "relation" {
                            for arg in ["parent", "child"] {
                                node_keys &= sc.tys[super::pos(&sc.names, super::s(&step.args()[arg])?)?] == Ty::Node;
                            }
                        } else {
                            node_keys &= sc.tys[super::pos(&sc.names, id)?] == Ty::Node;
                        }
                    }
                    node_keys
                }
                _ => false,
            }
        };
        local.push(here);
    }
    Ok(local)
}

pub fn partition(
    exec: &mut Exec<'_>,
    run: &mut Run,
    local: Vec<bool>,
) -> R<Partition> {
    let plan = exec.plan;
    let mut reached = vec![false; plan.nodes.len()];
    for &(_, output, _, _) in &plan.emits {
        reached[output] = true;
    }
    for (i, step) in plan.nodes.iter().enumerate().rev() {
        if reached[i] && !local[i] {
            for &source in &step.inputs {
                reached[source] = true;
            }
        }
    }
    let input_set: BTreeSet<_> = plan
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| reached[*i] && !local[*i])
        .flat_map(|(_, s)| s.inputs.iter().copied().filter(|i| local[*i]))
        .chain(plan.emits.iter().map(|e| e.1).filter(|i| local[*i]))
        .collect();
    let local_emit = plan.emits.iter().all(|(_, n, _, _)| local[*n]);
    // The reached closed steps that read each source, in step order: one pass over the plan, not one scan of it per source.
    let mut readers: Vec<Vec<usize>> = vec![Vec::new(); plan.nodes.len()];
    for (i, step) in plan.nodes.iter().enumerate().filter(|(i, _)| reached[*i] && !local[*i]) {
        for &source in &step.inputs {
            if readers[source].last() != Some(&i) {
                readers[source].push(i);
            }
        }
    }
    let mut columns = BTreeMap::new();
    for &source in &input_set {
        let Ok(schema) = exec.schema(run, source) else {
            columns.insert(source, Vec::new());
            continue;
        };
        let mut used = vec![false; schema.names.len()];
        if plan.emits.iter().any(|e| e.1 == source) {
            used.fill(true);
        }
        for (i, step) in readers[source].iter().map(|&i| (i, &plan.nodes[i])) {
            match step.op() {
                "select" => {
                    let Ok(select) = exec.sel(run, i) else {
                        used.fill(true);
                        break;
                    };
                    for (alias, _) in select
                        .srcs
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| **s == source)
                    {
                        for e in select
                            .conj
                            .iter()
                            .chain(select.items.iter().map(|(_, e)| e))
                        {
                            mark_columns(e, alias, &mut used);
                        }
                    }
                }
                "axis"
                    if step.inputs.iter().filter(|&&n| n == source).count() == 1
                        && exec.arg_rel(i, "relation") == Ok(source) =>
                {
                    if let (Ok(parent), Ok(child)) = (
                        super::pos(&schema.names, "parent"),
                        super::pos(&schema.names, "child"),
                    ) {
                        used[parent] = true;
                        used[child] = true;
                    } else {
                        used.fill(true);
                        break;
                    }
                }
                "antijoin"
                    if exec.arg_rel(i, "right") == Ok(source)
                        && exec.arg_rel(i, "left").is_ok_and(|s| s != source) =>
                {
                    let Ok(keys) = super::strs(&step.args()["right-on"]) else {
                        used.fill(true);
                        break;
                    };
                    for name in keys {
                        if let Ok(c) = super::pos(&schema.names, name) {
                            used[c] = true;
                        } else {
                            used.fill(true);
                            break;
                        }
                    }
                }
                _ => {
                    used.fill(true);
                    break;
                }
            }
        }
        columns.insert(source, used);
    }
    // A row can be dropped at the handoff only when every closed reader needs
    // a literal equality. Multiple aliases of one source are independent readers.
    let mut filters = BTreeMap::new();
    for &source in &input_set {
        if plan.nodes[source].op() == "base" || plan.emits.iter().any(|e| e.1 == source) {
            continue;
        }
        let mut alternatives = BTreeSet::new();
        let mut all_select = true;
        for (i, step) in readers[source].iter().map(|&i| (i, &plan.nodes[i])) {
            if step.op() != "select" {
                all_select = false;
                break;
            }
            let Ok(select) = exec.sel(run, i) else {
                all_select = false;
                break;
            };
            for (alias, _) in select
                .srcs
                .iter()
                .enumerate()
                .filter(|(_, n)| **n == source)
            {
                let term = select.conj.iter().find_map(|term| {
                    let E::Cmp(Op::Eq, left, right) = term else {
                        return None;
                    };
                    match (&**left, &**right) {
                        (E::Col(si, c, Ty::Sym), E::Text(literal))
                        | (E::Text(literal), E::Col(si, c, Ty::Sym))
                            if *si == alias =>
                        {
                            Some((*c, run.texts[*literal as usize].to_vec()))
                        }
                        _ => None,
                    }
                });
                match term {
                    Some(t) => {
                        alternatives.insert(t);
                    }
                    None => {
                        all_select = false;
                        break;
                    }
                }
            }
        }
        if all_select && !alternatives.is_empty() {
            filters.insert(source, alternatives.into_iter().collect());
        }
    }
    let mut domains = super::session::demand::domains(exec, run, &local, &input_set);
    for (source, keyed) in super::session::demand::walk_domains(exec, run, &local, &reached, &input_set) {
        domains.entry(source).or_insert(keyed);
    }
    let ancestor_domains = super::session::demand::ancestor_domains(exec, run, &local, &reached, &input_set);
    let reach = super::session::demand::reach_domains(exec, run, &local, &reached, &input_set);
    Ok(Partition {
        domains,
        ancestor_domains,
        reach,
        columns,
        filters,
        inputs: input_set.into_iter().collect(),
        file_steps: local,
        local_emit,
    })
}

pub fn mark_columns(expr: &E, alias: usize, used: &mut [bool]) {
    match expr {
        E::Col(source, col, _) => {
            if *source == alias {
                used[*col] = true;
            }
        }
        E::List(items) | E::Call(_, items) => {
            for item in items {
                mark_columns(item, alias, used);
            }
        }
        E::Cmp(_, a, b) | E::In(a, b, _) | E::And(a, b) | E::Or(a, b) => {
            mark_columns(a, alias, used);
            mark_columns(b, alias, used);
        }
        E::Not(x) | E::TextTest(x, _) => mark_columns(x, alias, used),
        E::Lit(_) | E::Text(_) | E::Num(_) => {}
    }
}
