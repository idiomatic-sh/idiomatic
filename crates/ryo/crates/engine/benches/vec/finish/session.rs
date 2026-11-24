//! Evaluate a closed set of typed populations once, after the final file.
#[path = "session_certificate.rs"]
pub(in super::super) mod certificate;
#[path = "session/demand.rs"]
pub(in super::super) mod demand;
#[path = "session/contribution.rs"]
pub(in super::super) mod contribution;
use super::population::Inputs;
use super::{E, Exec, File, FileVersion, NodeTable, Op, Provider, R, Run, Scope, Table, Ty, answer};
use std::{collections::BTreeMap, rc::Rc};

// Check the scalar subset before visiting any rows. Unimplemented functions on
// an empty input would otherwise escape the executor's runtime missing-function check.
fn scalar(expr: &E) -> R<()> {
    match expr {
        E::Call(name, args) => match name.as_str() {
            "starts_with" | "ends_with" | "concat" | "matches" | "glob_any" | "lower"
            | "snake" | "pascal" | "camel" | "kebab" => {
                args.iter().try_for_each(scalar)
            }
            _ => Err(format!("project scalar is unverified for {name}")),
        },
        E::In(a, b, _) | E::And(a, b) | E::Or(a, b) | E::Cmp(_, a, b) => {
            scalar(a)?;
            scalar(b)
        }
        E::Not(a) | E::TextTest(a, _) => scalar(a),
        E::List(items) => items.iter().try_for_each(scalar),
        E::Col(..) | E::Lit(_) | E::Text(_) | E::Num(_) => Ok(()),
    }
}

/// Absence-dependent steps retain only rows whose key is outside the incomplete files.
/// The filter runs at the affected step, before any later join or projection.
pub(in super::super) fn evaluate_covered(
    exec: &mut Exec<'_>,
    run: &mut Run,
    provider: &mut dyn Provider,
    admitted: &[u32],
    admit_file_subject: impl Fn(u32) -> bool,
    carried: &Inputs,
    incomplete: &BTreeMap<String, Vec<Option<u32>>>,
) -> R<Vec<answer::Row>> {
    let plan = exec.plan;
    // A supplied step has a consumer outside this operator chain. Optimizations
    // may read its materialization, but cannot substitute the inputs beneath it.
    if carried.keys().any(|node| !exec.emitted.contains(node)) {
        exec.emitted.extend(carried.keys().copied());
        exec.union_preps.fill(None);
        exec.keyed_forms.fill(None);
    }
    exec.run_memo.fill(None);
    run.missing.clear();
    // Each input spans the session. File-local views and implicit file equality
    // are inapplicable even when this particular request admits just one file.
    exec.closed_inputs();
    let mut demanded = vec![false; plan.nodes.len()];
    let mut stack: Vec<_> = plan.emits.iter().map(|e| e.1).collect();
    while let Some(i) = stack.pop() {
        if std::mem::replace(&mut demanded[i], true) || carried.contains_key(&i) {
            continue;
        }
        stack.extend(&plan.nodes[i].inputs);
    }
    for (&i, input) in carried {
        let sc=exec.schema(run,i)?;
        if !input.agrees(&sc.names,&sc.tys) {
            return Err("frontier schema differs from its prepared step".into());
        }
    }
    let mut bases = BTreeMap::<&str, Rc<Table>>::new();
    for (i, step) in plan.nodes.iter().enumerate() {
        if !demanded[i] || carried.contains_key(&i) {
            continue;
        }
        match step.op.as_str() {
            "base" => {
                let rel = step.base().ok_or("missing base relation")?;
                let table = match bases.get(rel) {
                    Some(t) => Rc::clone(t),
                    None => {
                        let (_, names, tys) = run
                            .catalog(rel)
                            .ok_or_else(|| format!("undeclared relation {rel}"))?;
                        let t = provider.shared_fill(run, rel, Scope::Files(admitted))?;
                        if t.names != names
                            || t.tys != tys
                            || t.cols.len() != names.len()
                            || t.cols.iter().any(|col| col.len() != t.n)
                        {
                            return Err(format!(
                                "project population disagrees with catalog for {rel}"
                            ));
                        }
                        if tys.iter().any(|ty| matches!(ty, Ty::Node)) {
                            return Err(format!("{rel} requires project source origins"));
                        }
                        for (ty, column) in tys.iter().zip(&t.cols) {
                            let limit = match ty {
                                Ty::File => run.paths.len() as u64,
                                Ty::Sym => run.texts.len() as u64,
                                Ty::Flag => 2,
                                // A span is the published pair of byte offsets.
                                Ty::Span => continue,
                                _ => super::NULL,
                            };
                            if column.iter().any(|word| *word != super::NULL && *word >= limit) {
                                return Err(format!("unbound or absent {ty:?} cell in {rel}"));
                            }
                        }
                        let _previous = bases.insert(rel, Rc::clone(&t));
                        t
                    }
                };
                exec.run_memo[i] = Some(table);
            }
            "rows" | "select" | "project" | "union" | "antijoin" | "group-count" | "words" | "axis" | "closure" | "cycles" | "rank" | "intersection" | "difference" | "group-digest" | "replace" | "span-bounds" => {}
            "extend"
                if step.args()["values"]
                    .as_object()
                    .is_some_and(|m| m.len() <= 1)
                    || step.args()["value-order"].as_array().is_some() => {}
            "extend" => {
                return Err("project emit needs declaration order for multi-column extend".into());
            }
            "capture" if step.args()["separator"].as_str().is_some() => {}
            "capture" => return Err("project capture needs a verified separator contract".into()),
            op => return Err(format!("project scope is unverified for {op}")),
        }
    }

    for (i, step) in plan.nodes.iter().enumerate() {
        if !demanded[i] || carried.contains_key(&i) {
            continue;
        }
        let _schema = exec.schema(run, i)?;
        match step.op.as_str() {
            "select" => {
                let select = exec.sel(run, i)?;
                for expr in select
                    .conj
                    .iter()
                    .chain(select.items.iter().map(|(_, expr)| expr))
                {
                    scalar(expr)?;
                }
            }
            "antijoin" => {
                let _prepared = super::antijoin_kernel::prep(exec, run, i)?;
            }
            "union" => {
                let _prepared = super::union_kernel::prep(exec, run, i)?;
            }
            "group-count" => {
                if let Some(name) = step.args()["distinct"].as_str() {
                    let input = exec.arg_rel(i, "from")?;
                    let schema = exec.schema(run, input)?;
                    let _column = super::pos(&schema.names, name)?;
                }
            }
            _ => {}
        }
    }
    super::subject_scope::restrict(exec,run,carried,&admit_file_subject)?;
    let scratch = NodeTable::default();
    let version = FileVersion::new(&scratch, &[], u32::MAX, None);
    let mut frame = File::new(&version, plan.nodes.len());
    frame.after_last_file = true;
    frame.project_inputs = Some(carried);
    // A supplied population is a materialized input, including for keyed and closure
    // kernels, which consult the frame before deciding to visit an input's children.
    for (node, table) in exec.run_memo.iter().enumerate() {
        if let Some(table) = table {
            frame.remember(node, super::Res::Tab(Rc::clone(table)));
        }
    }
    if !incomplete.is_empty() {
        restrict_uncertain(exec, run, provider, &mut frame, &demanded, incomplete)?;
    }
    let mut rows = Vec::new();
    for (emit, (rule, node, subject, keys)) in plan.emits.iter().enumerate() {
        let t = exec.eval(&mut frame, run, provider, *node)?;
        if !run.missing.is_empty() {
            return Err(format!("unimplemented functions {:?}", run.missing));
        }
        let layout = exec.answer_layout(run, (0, emit), rule, &t, subject, keys)?;
        let mut seen = BTreeMap::new();
        for i in 0..t.n {
            let subject = layout.identity[0];
            if matches!(t.tys[subject], Ty::File | Ty::Node) {
                let cell = t.cols[subject][i];
                if cell == super::NULL { continue; }
                let id = if t.tys[subject] == Ty::Node {
                    cell >> 32
                } else {
                    cell
                };
                let file = u32::try_from(id).map_err(|e| e.to_string())?;
                if !admit_file_subject(file) {
                    continue;
                }
            }
            let row = answer::typed(run, &t, i, None, &layout)?;
            if t.n == 1 { rows.push(row); } else { keep_finding(&mut seen, row); }
        }
        rows.extend(seen.into_values());
    }
    Ok(rows)
}

/// A join can produce multiple rows for one public finding. Its placed key and
/// subject coincide, so the lexicographically first typed payload wins the tie.
fn keep_finding(
    seen: &mut BTreeMap<Vec<answer::Value>, answer::Row>,
    row: answer::Row,
) {
    use std::collections::btree_map::Entry;
    let key = row.layout.identity.iter().map(|&i| row.value(i)).collect();
    match seen.entry(key) {
        Entry::Vacant(slot) => { slot.insert(row); }
        Entry::Occupied(mut slot) => {
            if row.cells().cmp(slot.get().cells()).is_lt() {
                slot.insert(row);
            }
        }
    }
}

fn restrict_uncertain(
    exec: &mut Exec<'_>,
    run: &mut Run,
    provider: &mut dyn Provider,
    frame: &mut File<'_>,
    demanded: &[bool],
    inputs: &BTreeMap<String, Vec<Option<u32>>>,
) -> R<()> {
    let mut affected = Vec::<std::collections::BTreeSet<Option<u32>>>::new();
    for (i, step) in exec.plan.nodes.iter().enumerate() {
        let mut here = std::collections::BTreeSet::new();
        if let Some(files) = step.base().and_then(|n| inputs.get(n)) {
            here.extend(files);
        }
        for &source in &step.inputs {
            here.extend(&affected[source]);
        }
        if demanded[i] {
            let probe = match step.op.as_str() {
                "group-count" | "group-digest" => Some((exec.arg_rel(i, "from")?, "keys")),
                "difference" => Some((exec.arg_rel(i, "right")?, "")),
                "antijoin" => Some((exec.arg_rel(i, "right")?, "left-on")),
                _ => None,
            };
            if let Some((source, keys)) = probe
                && !affected[source].is_empty()
            {
                if !frame.after_last_file {
                    // A negative step inside one incomplete source cannot publish any rows.
                    frame.remember(i, super::Res::Tab(exec.empty_rc(run, i)?));
                    affected.push(here);
                    continue;
                }
                let out = exec.eval(frame, run, provider, i)?;
                let mut file_key = if keys.is_empty() {
                    out.tys.iter().position(|ty| matches!(ty, Ty::File | Ty::Node))
                } else { None };
                for key in super::strs(&step.args()[keys])? {
                    let col = out.col(key)?;
                    if matches!(out.tys[col], Ty::File | Ty::Node) {
                        file_key = Some(col);
                        break;
                    }
                }
                let mut kept = Table::with_rows(out.names.clone(), out.tys.clone(), out.n);
                if let Some(key) = file_key
                    && !affected[source].contains(&None)
                {
                    for row in 0..out.n {
                        let word = out.cols[key][row];
                        let file = if out.tys[key] == Ty::Node {
                            word >> 32
                        } else {
                            word
                        };
                        if word != super::NULL
                            && !affected[source].contains(&u32::try_from(file).ok())
                        {
                            for (dest, c) in kept.cols.iter_mut().zip(&out.cols) {
                                dest.push(c[row]);
                            }
                            kept.n += 1;
                        }
                    }
                }
                let kept = Rc::new(kept);
                frame.remember(i, super::Res::Tab(kept.clone()));
                exec.run_memo[i] = Some(kept);
            }
        }
        affected.push(here);
    }
    Ok(())
}

/// Run one already prepared file program and deliver its complete typed rows. Caller
/// tables have already been numbered in this request and belong to this binding.
pub(in super::super) fn frame<'a>(
    version: &'a FileVersion<'a>,
    plan: &super::Plan,
    supplied: &[(String, Rc<Table>)],
    local: Option<&(String, Rc<Table>)>,
) -> File<'a> {
    let mut frame = File::new(version, plan.nodes.len());
    frame.fills.extend(supplied.iter().chain(local).cloned());
    frame
}

pub(in super::super) fn pair(
    exec: &mut Exec<'_>,
    frame: &mut File<'_>,
    run: &mut Run,
    facts: &mut dyn Provider,
    plan: &super::Plan,
    binding: usize,
    slots: &[usize],
    incomplete: &BTreeMap<String, Vec<Option<u32>>>,
) -> R<Vec<answer::Row>> {
    if !incomplete.is_empty() {
        let mut demanded = vec![false; exec.plan.nodes.len()];
        for &slot in slots { demanded[slot] = true; }
        restrict_uncertain(exec, run, facts, frame, &demanded, incomplete)?;
    }
    let mut rows = Vec::new();
    for (emit, (rule, node, subject, keys)) in plan.emits.iter().enumerate() {
        run.missing.clear();
        let t = exec.eval(frame, run, facts, slots[*node])?;
        if !run.missing.is_empty() {
            return Err(format!("unimplemented functions {:?}", run.missing));
        }
        let layout = exec.answer_layout(run, (binding, emit), rule, &t, subject, keys)?;
        let mut seen = BTreeMap::new();
        for r in 0..t.n {
            let row = answer::typed(run, &t, r, Some(frame.file as usize), &layout)?;
            if t.n == 1 { rows.push(row); } else { keep_finding(&mut seen, row); }
        }
        rows.extend(seen.into_values());
    }
    Ok(rows)
}

/// Evaluate just the local outputs consumed by a wider step. Their types and
/// multiplicities are identical to the producer's ordinary result.
pub(in super::super) fn gather(
    exec: &mut Exec<'_>,
    run: &mut Run,
    frame: &mut File<'_>,
    facts: &mut dyn Provider,
    slots: &[usize],
    cut: &[usize],
    projections: &BTreeMap<usize,contribution::Projection>,
    incomplete: &BTreeMap<String, Vec<Option<u32>>>,
) -> R<Vec<(usize, Rc<Table>, bool)>> {
    if !incomplete.is_empty() {
        let mut demanded = vec![false; exec.plan.nodes.len()];
        let mut stack: Vec<_> = cut.iter().map(|&i| slots[i]).collect();
        while let Some(i) = stack.pop() {
            if !std::mem::replace(&mut demanded[i], true) {
                stack.extend(&exec.plan.nodes[i].inputs);
            }
        }
        restrict_uncertain(exec, run, facts, frame, &demanded, incomplete)?;
    }
    let mut result = Vec::with_capacity(cut.len());
    run.missing.clear();
    for &i in cut {
        if incomplete.is_empty()
            && let Some(table) = contribution::gather(projections, i, slots, frame, run) {
            result.push((i, table, true));
        } else {
            result.push((i, exec.eval(frame, run, facts, slots[i])?, false));
        }
    }
    if !run.missing.is_empty() {
        return Err(format!("unimplemented functions {:?}", run.missing));
    }
    Ok(result)
}
