//! Evaluate a closed set of typed populations once, after the final file.
use super::{E, Exec, File, NodeTable, Op, Plan, Provider, R, Run, Scope, Table, Ty, answer};
use std::{collections::BTreeMap, rc::Rc};

// Check the scalar subset before visiting any rows. Unimplemented functions on
// an empty input would otherwise escape the executor's runtime missing-function check.
fn scalar(expr: &E) -> R<()> {
    match expr {
        E::Seed(..) => Err("project predicate requires file-local seeds".into()),
        E::Cmp(Op::Other, ..) => Err("unsupported project comparison".into()),
        E::Call(name, args) => match name.as_str() {
            "starts_with" | "ends_with" | "concat" | "matches" | "glob_any" | "lower" => {
                args.iter().try_for_each(scalar)
            }
            _ => Err(format!("project scalar is unverified for {name}")),
        },
        E::In(a, b, _) | E::And(a, b) | E::Or(a, b) | E::Cmp(_, a, b) => {
            scalar(a)?;
            scalar(b)
        }
        E::Null(a, _) | E::Not(a) | E::Re(a, _) => scalar(a),
        E::List(items) => items.iter().try_for_each(scalar),
        E::Col(..) | E::Lit(_) | E::Text(_) | E::Num(_) => Ok(()),
    }
}

/// Complete even for an empty admission. Resolve every base before evaluating an
/// emit, so a short-circuit or an antijoin cannot turn an unavailable input into evidence.
pub(in super::super) fn evaluate(
    plan: &Plan,
    run: &mut Run,
    provider: &mut dyn Provider,
    admitted: &[u32],
    admit_file_subject: impl Fn(u32) -> bool,
) -> R<Vec<String>> {
    if !plan.seeds.is_empty() {
        return Err("project scope requires complete populations for survivor seeds".into());
    }
    let mut exec = Exec::new(plan);
    // Each input spans the session. File-local views and implicit file equality
    // are inapplicable even when this particular request admits just one file.
    exec.views.fill(Some(None));
    exec.constant.fill(Some(true));
    // A closed session asks once for all complete populations, even when the
    // admitted set is empty. Retain the first occurrence order of the plan.
    let mut ordered = Vec::new();
    let mut slots = BTreeMap::new();
    for step in &plan.nodes {
        if step.op != "base" { continue; }
        let rel = step.base().ok_or("missing base relation")?;
        if let std::collections::btree_map::Entry::Vacant(entry) = slots.entry(rel) {
            entry.insert(ordered.len());
            ordered.push(super::request::Asked { rel, columns: &ryo::languages::ColumnProjection::All, scope: Scope::Files(admitted) });
        }
    }
    let mut answer = Vec::with_capacity(ordered.len());
    provider.answer(run, &super::request::Request { relations: &ordered }, &mut answer)?;
    if ordered.len() != answer.len() { return Err("incomplete project answer".into()); }
    let mut answers = answer.into_iter().map(Some).collect::<Vec<_>>();
    let mut bases = BTreeMap::<&str, Rc<Table>>::new();
    for (i, step) in plan.nodes.iter().enumerate() {
        match step.op.as_str() {
            "base" => {
                let rel = step.base().ok_or("missing base relation")?;
                let table = match bases.get(rel) {
                    Some(t) => Rc::clone(t),
                    None => {
                        let (_, names, tys) = run
                            .catalog(rel)
                            .ok_or_else(|| format!("undeclared relation {rel}"))?;
                        let t = answers[slots[rel]].take().expect("first use of a closed population")?;
                        if t.names != names
                            || t.tys != tys
                            || t.cols.len() != names.len()
                            || t.cols.iter().any(|col| col.len() != t.n)
                        {
                            return Err(format!(
                                "project population disagrees with catalog for {rel}"
                            ));
                        }
                        if tys.iter().any(|ty| matches!(ty, Ty::Node | Ty::Span)) {
                            return Err(format!("{rel} requires project source origins"));
                        }
                        for (ty, column) in tys.iter().zip(&t.cols) {
                            let limit = match ty {
                                Ty::File => run.paths.len() as u64,
                                Ty::Sym => run.texts.len() as u64,
                                Ty::Flag => 2,
                                _ => super::NULL,
                            };
                            if column.iter().any(|word| *word >= limit) {
                                return Err(format!("unbound or absent {ty:?} cell in {rel}"));
                            }
                        }
                        let t = Rc::new(t);
                        let _previous = bases.insert(rel, Rc::clone(&t));
                        t
                    }
                };
                exec.run_memo[i] = Some(table);
            }
            "rows" | "select" | "project" | "union" | "antijoin" | "group-count" => {}
            "extend"
                if step.args()["values"]
                    .as_object()
                    .is_some_and(|m| m.len() <= 1) => {}
            "extend" => {
                return Err("project emit needs declaration order for multi-column extend".into());
            }
            "capture" => return Err("project capture needs a verified separator contract".into()),
            op => return Err(format!("project scope is unverified for {op}")),
        }
    }

    for (i, step) in plan.nodes.iter().enumerate() {
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
                let _prepared = super::antijoin_kernel::prep(&mut exec, run, i)?;
            }
            "union" => {
                let _prepared = super::union_kernel::prep(&mut exec, run, i)?;
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
    let scratch = NodeTable::default();
    let mut frame = File::new(&scratch, &[], u32::MAX, plan.nodes.len(), run.fact_offsets.clone());
    frame.after_last_file = true;
    let mut rows = Vec::new();
    for (rule, node, subject, keys) in &plan.emits {
        let t = exec.eval(&mut frame, run, provider, *node)?;
        if !run.missing.is_empty() {
            return Err(format!("unimplemented functions {:?}", run.missing));
        }
        let identity = std::iter::once(subject)
            .chain(keys)
            .map(|name| t.col(name))
            .collect::<R<Vec<_>>>()?;
        let mut seen = BTreeMap::new();
        for i in 0..t.n {
            let subject = identity[0];
            if t.tys[subject] == Ty::File {
                let file = u32::try_from(t.cols[subject][i]).map_err(|e| e.to_string())?;
                if !admit_file_subject(file) {
                    continue;
                }
            }
            let key: Vec<_> = identity.iter().map(|c| t.cols[*c][i]).collect();
            let row = answer::row(run, &t, i, rule, None, &identity)?;
            if let Some(previous) = seen.insert(key, row.clone())
                && previous != row
            {
                return Err("conflicting emitted values for one subject and key".into());
            }
        }
        rows.extend(seen.into_values());
    }
    rows.sort_unstable();
    Ok(rows)
}
