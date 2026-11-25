//! Evaluate a closed set of typed populations once, after the final file.
#[path = "session_certificate.rs"]
pub(in super::super) mod certificate;
#[path = "session/demand.rs"]
pub(in super::super) mod demand;
#[path = "session/contribution.rs"]
pub(in super::super) mod contribution;
#[path = "session/reuse.rs"]
pub(in super::super) mod reuse;
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
    mut memo: Option<(&mut reuse::Reuse, [usize; 3])>,
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
    // A file can publish an incomplete relation unrelated to an ordered operator.
    // Follow the actual operands, including the lineage of a carried frontier.
    let mut uncertain = Vec::new();
    if !incomplete.is_empty() {
        for step in &plan.nodes {
            uncertain.push(step.base().is_some_and(|rel| incomplete.contains_key(rel))
                || step.inputs.iter().any(|&source| uncertain[source]));
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
            // Class numbers are interned across the complete input. A local class
            // from one file has no meaning in another file's class dictionary.
            "subtree-class" | "order" if !uncertain.get(i).copied().unwrap_or(false) => {}
            "as-of" | "rows" | "select" | "project" | "union" | "antijoin" | "group-count" | "words" | "axis" | "closure" | "cycles" | "rank" | "intersection" | "difference" | "group-digest" | "replace" | "span-bounds" | "span-between" => {}
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

    let complete = incomplete.is_empty();
    let mut zero = vec![false; plan.nodes.len()];
    let mut deps = if complete && memo.as_ref().is_some_and(|(state, _)| state.enabled) { vec![reuse::VARIABLE; plan.nodes.len()] } else { Vec::new() };
    for (i, step) in plan.nodes.iter().enumerate() {
        if let Some(input) = carried.get(&i) {
            zero[i] = input.n == 0;
            continue;
        }
        if !demanded[i] {
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
        if complete {
            zero[i] = exec.run_memo[i].as_ref().map_or_else(
                || reuse::empty(step, &zero),
                |table| table.n == 0,
            );
            if !deps.is_empty() {
                let mask = match step.base() {
                    Some("filesystem.file") => reuse::INVENTORY,
                    Some(name) if matches!(run.supplier(name), super::Supplier::Host) => reuse::HOST | reuse::INVENTORY,
                    Some(_) => reuse::VARIABLE,
                    None => step.inputs.iter().fold(0, |mask, &j| mask | (deps[j] & !reuse::RESTORED)),
                };
                deps[i] = mask;
                if step.op != "base" && !zero[i]
                    && let Some((memo, scope)) = memo.as_mut()
                    && let Some(table) = memo.get(i, mask, *scope)
                {
                    zero[i] = table.n == 0;
                    exec.run_memo[i] = Some(table);
                    deps[i] |= reuse::RESTORED;
                }
            }
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
        if let Some(table) = table
            && deps.get(node).is_none_or(|d| d & reuse::RESTORED == 0)
        {
            frame.remember(node, super::Res::Tab(Rc::clone(table)));
        }
    }
    if !incomplete.is_empty() {
        restrict_uncertain(exec, run, provider, &mut frame, &demanded, incomplete)?;
    }
    let mut rows = Vec::new();
    for (emit, (rule, node, subject, keys)) in plan.emits.iter().enumerate() {
        let t = if complete && zero[*node] && frame.memo[*node].is_none() {
            if let Some((memo, _)) = memo.as_mut() { memo.empty_emits += 1; }
            exec.empty_rc(run, *node)?
        } else {
            exec.eval(&mut frame, run, provider, *node)?
        };
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
    if let Some((memo, scope)) = memo {
        memo.remember(&deps, scope, &frame.memo);
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

/// The input whose absence can invalidate a produced row and the output file keys.
pub(in super::super) fn absence_input(step: &super::PNode) -> Option<(&'static str, &'static str)> {
    match step.op.as_str() {
        "group-count" | "group-digest" => Some(("from", "keys")),
        "difference" => Some(("right", "")),
        "antijoin" | "as-of" => Some(("right", "left-on")),
        _ => None,
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
            let probe = absence_input(step).map(|(source, keys)| exec.arg_rel(i, source).map(|at| (at, keys))).transpose()?;
            if let Some((source, keys)) = probe
                && !affected[source].is_empty()
            {
                if !frame.after_last_file {
                    // A negative step inside one incomplete source cannot publish any rows.
                    frame.remember(i, super::Res::Tab(exec.empty_rc(run, i)?));
                    affected.push(here);
                    continue;
                }
                if step.op == "as-of" {
                    // Exclude uncertain partitions before choosing the nearest row.
                    // The projection may discard or rename its partition key.
                    let file_local_right = frame.project_inputs.is_some_and(|xs| xs.contains_key(&source));
                    let result = if file_local_right && !affected[source].contains(&None) {
                        exec.ordered(frame, run, provider, i, step.args(), true, Some(&affected[source]))?
                    } else {
                        exec.empty(run, i)?
                    };
                    let result = Rc::new(result);
                    frame.remember(i, super::Res::Tab(Rc::clone(&result)));
                    exec.run_memo[i] = Some(result);
                    affected.push(here);
                    continue;
                }
                let out = exec.eval(frame, run, provider, i)?;
                let mut file_key = if keys.is_empty() {
                    out.tys.iter().position(|ty| matches!(ty, Ty::File | Ty::Node))
                } else { None };
                for key in (!keys.is_empty()).then(|| super::strs(&step.args()[keys])).transpose()?.into_iter().flatten() {
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
pub(in super::super) fn frame<'a, 't>(
    version: &'a FileVersion<'a>,
    plan: &super::Plan,
    supplied: &[(String, Rc<Table>)],
    local: impl IntoIterator<Item = &'t (String, Rc<Table>)>,
) -> File<'a> {
    let mut frame = File::new(version, plan.nodes.len());
    frame.fills.extend(supplied.iter().cloned());
    frame.fills.extend(local.into_iter().cloned());
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
        publish_table(exec, run, (binding, emit), rule, subject, keys, &t, frame.file as usize, &mut rows)?;
    }
    Ok(rows)
}

/// Once the pair has run successfully, an incomplete input is immaterial if
/// every emit requires a row from an independently complete, empty input.
/// Consume the same necessary-input edges used by the execution frame.
pub(crate) fn covered_empty(
    exec: &mut Exec<'_>, frame: &mut File<'_>, run: &mut Run, facts: &mut dyn Provider,
    plan: &super::Plan, slots: &[usize], complete: impl Fn(&str) -> Option<usize>,
) -> bool {
    let mut empty = vec![false; exec.plan.nodes.len()];
    let mut known = vec![false; empty.len()];
    for (step, &slot) in plan.nodes.iter().zip(slots) {
        if let Some(name) = step.base() {
            if let Some(count) = complete(name) {
                known[slot] = true;
                empty[slot] = count == 0;
            }
            continue;
        }
        let need = exec.need(run, slot);
        if !need.never && !need.inputs.is_empty() {
            empty[slot] = if need.all {
                need.inputs.iter().all(|&i| empty[i])
            } else {
                need.inputs.iter().any(|&i| empty[i])
            };
        }
        // These relational operators depend only on their inputs. Use their
        // actual evaluated rows, including filters over a complete named base.
        if !empty[slot] && matches!(step.op.as_str(), "select" | "project")
            && !step.inputs.is_empty() && step.inputs.iter().all(|&i| known[slots[i]])
        {
            if step.op == "select" && !exec.sel(run, slot).is_ok_and(|select|
                select.conj.iter().chain(select.items.iter().map(|(_, expr)| expr)).all(certificate::total)) {
                continue;
            }
            known[slot] = true;
            if let Ok(table) = exec.eval(frame, run, facts, slot) {
                empty[slot] = table.n == 0;
            } else {
                return false;
            }
        }
        known[slot] |= empty[slot];
    }
    !plan.emits.is_empty() && plan.emits.iter().all(|emit| empty[slots[emit.1]])
}

/// Publish an emit identically whether its table came from evaluation or a complete
/// location proof. Deduplication is confined to that emit in that file.
pub(in super::super) fn publish_table(
    exec: &mut Exec<'_>, run: &mut Run, slot: (usize, usize),
    rule: &str, subject: &str, keys: &[String], table: &Table, file: usize, rows: &mut Vec<answer::Row>,
) -> R<()> {
    let layout = exec.answer_layout(run, slot, rule, table, subject, keys)?;
    let mut seen = BTreeMap::new();
    for r in 0..table.n {
        let row = answer::typed(run, table, r, Some(file), &layout)?;
        if table.n == 1 { rows.push(row); } else { keep_finding(&mut seen, row); }
    }
    rows.extend(seen.into_values());
    Ok(())
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
