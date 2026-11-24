//! Locate the outputs of file-local steps consumed after admission closes.
//! Every node number addresses the same prepared executor plan in both phases.
use super::{E, Exec, Op, R, Run, Ty};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Default)]
pub(in super::super) struct Partition {
    pub inputs: Vec<usize>,
    pub columns: BTreeMap<usize, Vec<bool>>,
    pub filters: BTreeMap<usize, Vec<(usize, Vec<u8>)>>,
    pub local_emit: bool,
    pub file_steps: Vec<bool>,
}

/// A multidirectional join stays local only if its equality edges connect every
/// source with file-local keys. Pure project relations and caller snapshots are
/// always complete at the later phase, even if they happen to hold one row.
pub(in super::super) fn partition(
    exec: &mut Exec<'_>,
    run: &mut Run,
    wide: impl Fn(&str) -> bool,
) -> R<Partition> {
    exec.closed_inputs();
    // All operands here are already local. A span names a place in that source.
    fn anchor(ty: Ty) -> bool {
        matches!(ty, Ty::Node | Ty::File | Ty::Span)
    }
    let plan = exec.plan;
    let mut local = Vec::with_capacity(plan.nodes.len());
    for (i, step) in plan.nodes.iter().enumerate() {
        let args = step.args();
        let keys_hold = |ex: &mut Exec<'_>, run: &mut Run, from: &str, keys: &str| -> R<bool> {
            let source = ex.arg_rel(i, from)?;
            let s = ex.schema(run, source)?;
            for name in super::strs(&args[keys])? {
                if anchor(s.tys[super::pos(&s.names, name)?]) {
                    return Ok(true);
                }
            }
            Ok(false)
        };
        let here = if !step.inputs.iter().all(|&source| local[source]) {
            false
        } else {
            match step.op.as_str() {
                "base" => !wide(step.base().ok_or("base relation")?),
                "rows" => false,
                "select" => {
                    if step.inputs.len() < 2 {
                        true
                    } else {
                        let names = step
                            .inputs
                            .iter()
                            .enumerate()
                            .map(|(i, &n)| {
                                Ok((step.alias(i).map(str::to_owned), exec.schema(run, n)?))
                            })
                            .collect::<R<Vec<_>>>()?;
                        let mut components: Vec<_> = (0..names.len()).collect();
                        let mut terms = Vec::new();
                        if let Some(cond) = exec.select_condition(run, step, &names)? {
                            super::flatten(cond, &mut terms);
                        }
                        for term in terms {
                            let E::Cmp(Op::Eq, a, b) = term else { continue };
                            let (E::Col(left, _, lt), E::Col(right, _, rt)) = (*a, *b) else {
                                continue;
                            };
                            if lt != rt || !anchor(lt) {
                                continue;
                            }
                            let (a, b) = (components[left], components[right]);
                            for c in &mut components {
                                if *c == b {
                                    *c = a;
                                }
                            }
                        }
                        components.iter().all(|&c| c == components[0])
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
                "antijoin" | "as-of" | "ordered-match" => {
                    keys_hold(exec, run, "left", "left-on")?
                        && keys_hold(exec, run, "right", "right-on")?
                }
                "group-count" | "group-digest" | "group-min" | "group-max" | "group-ordinal" => {
                    keys_hold(exec, run, "from", "keys")?
                }
                "rank" | "extend" | "capture" | "span-bounds" | "span-between"
                | "source-matches" | "render-syntax" | "fits-rendered" | "words" | "replace" => {
                    true
                }
                "closure" | "cycles" | "follow" => {
                    let source = exec.arg_rel(i, "edges")?;
                    let edges = exec.schema(run, source)?;
                    let source = super::pos(&edges.names, super::s(&args["source"])?)?;
                    anchor(edges.tys[source])
                        || (step.op.as_str() != "cycles" && keys_hold(exec, run, "edges", "keys")?)
                }
                "axis" => true,
                _ => false,
            }
        };
        local.push(here);
    }
    let input_set: BTreeSet<_> = plan
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| !local[*i])
        .flat_map(|(_, s)| s.inputs.iter().copied().filter(|i| local[*i]))
        .chain(plan.emits.iter().map(|e| e.1).filter(|i| local[*i]))
        .collect();
    let local_emit = plan.emits.iter().all(|(_, n, _, _)| local[*n]);
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
        for (i, step) in plan
            .nodes
            .iter()
            .enumerate()
            .filter(|(i, s)| !local[*i] && s.inputs.contains(&source))
        {
            match step.op.as_str() {
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
        if plan.nodes[source].op == "base" || plan.emits.iter().any(|e| e.1 == source) {
            continue;
        }
        let mut alternatives = BTreeSet::new();
        let mut all_select = true;
        for (i, step) in plan
            .nodes
            .iter()
            .enumerate()
            .filter(|(i, s)| !local[*i] && s.inputs.contains(&source))
        {
            if step.op != "select" {
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
    Ok(Partition {
        columns,
        filters,
        inputs: input_set.into_iter().collect(),
        file_steps: local,
        local_emit,
    })
}

fn mark_columns(expr: &E, alias: usize, used: &mut [bool]) {
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
        E::Null(x, _) | E::Not(x) | E::TextTest(x, _) | E::Seed(x, _) => mark_columns(x, alias, used),
        E::Lit(_) | E::Text(_) | E::Num(_) => {}
    }
}
