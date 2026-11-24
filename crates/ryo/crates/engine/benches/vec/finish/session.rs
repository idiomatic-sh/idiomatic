//! Evaluate a closed set of typed populations once, after the final file.
use super::{E, Exec, File, NodeTable, Op, Provider, R, Run, Scope, Table, Ty, json_output};
use std::{collections::BTreeMap, rc::Rc};

// Check the scalar subset before visiting any rows. Unimplemented functions on
// an empty input would otherwise escape the executor's runtime missing-function check.
fn scalar(expr: &E) -> R<()> {
    match expr {
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
        E::Not(a) | E::Re(a, _) => scalar(a),
        E::List(items) => items.iter().try_for_each(scalar),
        E::Col(..) | E::Lit(_) | E::Text(_) | E::Num(_) => Ok(()),
    }
}

/// Complete even for an empty admission. Resolve every base before evaluating an
/// emit, so a short-circuit or an antijoin cannot turn an unavailable input into evidence.
pub(in super::super) fn evaluate(
    exec: &mut Exec<'_>,
    run: &mut Run,
    provider: &mut dyn Provider,
    admitted: &[u32],
    admit_file_subject: impl Fn(u32) -> bool,
) -> R<json_output::Records> {
    let plan = exec.plan;
    exec.run_memo.fill(None);
    run.missing.clear();
    // Each input spans the session. File-local views and implicit file equality
    // are inapplicable even when this particular request admits just one file.
    exec.views.fill(Some(None));
    exec.constant.fill(Some(true));
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
                        let t = provider.fill(run, rel, Scope::Files(admitted))?;
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
    let scratch = NodeTable::default();
    let mut frame = File::new(&scratch, &[], u32::MAX, plan.nodes.len());
    frame.after_last_file = true;
    let mut rows = json_output::Records::default();
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
            if let Some(previous) = seen.insert(key, i) {
                for (ty, column) in t.tys.iter().zip(&t.cols) {
                    let normalized = |word: u64| match ty {
                        Ty::Sym if run.absent_word == Some(word as u32) => super::NULL,
                        Ty::Flag if word != super::NULL => u64::from(word != 0),
                        _ => word,
                    };
                    if normalized(column[previous]) != normalized(column[i]) {
                        return Err("conflicting emitted values for one subject and key".into());
                    }
                }
            }
        }
        for row in seen.into_values() {
            rows.query(run, &t, row, rule, None, &identity)?;
        }
    }
    Ok(rows)
}

/// Run one already prepared file program and deliver its complete typed rows. Caller
/// tables have already been numbered in this request and belong to this binding.
pub(in super::super) fn pair<'tables>(
    exec: &mut Exec<'_>,
    run: &mut Run,
    path: u32,
    tree: &NodeTable,
    bytes: &[u8],
    facts: &mut dyn Provider,
    supplied: impl IntoIterator<Item = &'tables (String, Rc<Table>)>,
) -> R<json_output::Records> {
    let plan = exec.plan;
    let mut frame = File::new(tree, bytes, path, plan.nodes.len());
    if let Some(text) = facts.syntax_text() {
        frame.set_text(text);
    }
    for (name, table) in supplied {
        if run
            .catalog(name)
            .is_none_or(|(_, names, tys)| names != table.names || tys != table.tys)
        {
            let fields: Vec<_> = table
                .names
                .iter()
                .zip(table.tys.iter())
                .map(|(n, t)| (n.as_str(), *t))
                .collect();
            run.declare(name, &fields, super::Supplier::Host);
        }
        frame.fills.insert(name.clone(), table.clone());
    }
    let mut rows = json_output::Records::default();
    for (rule, node, subject, keys) in &plan.emits {
        run.missing.clear();
        let t = exec.eval(&mut frame, run, facts, *node)?;
        if !run.missing.is_empty() {
            return Err(format!("unimplemented functions {:?}", run.missing));
        }
        let identity = std::iter::once(subject)
            .chain(keys)
            .map(|name| t.col(name))
            .collect::<R<Vec<_>>>()?;
        let mut seen = std::collections::BTreeSet::new();
        for r in 0..t.n {
            let key: Vec<_> = identity.iter().map(|c| t.cols[*c][r]).collect();
            if seen.insert(key) {
                rows.query(
                    run,
                    &t,
                    r,
                    rule,
                    Some(path as usize),
                    &identity,
                )?;
            }
        }
    }
    Ok(rows)
}
