//! Check that skipping an empty local emit also preserves evaluation errors.
use super::super as finish;
use finish::{E, Exec, R, Run, Table, Ty};
use std::rc::Rc;

// Dynamic pattern compilation can refuse a non-empty intermediate even
// when the downstream emit is empty. A literal pattern is already E::TextTest.
pub(in super::super) fn total(expr: &E) -> bool {
    match expr {
        E::Call(name, args) => {
            let arity = match name.as_str() {
                "lower" => args.len() == 1,
                "starts_with" | "ends_with" => args.len() == 2,
                "concat" => true,
                _ => false,
            };
            arity && args.iter().all(total)
        }
        E::In(a, b, _) | E::And(a, b) | E::Or(a, b) | E::Cmp(_, a, b) => total(a) && total(b),
        E::Not(a) | E::TextTest(a, _) => total(a),
        E::List(items) => items.iter().all(total),
        E::Col(..) | E::Lit(_) | E::Text(_) | E::Num(_) => true,
    }
}

/// A missing witness can skip evaluation once the binding has passed these checks.
pub(crate) fn certified_empty(
    exec: &mut Exec<'_>,
    run: &mut Run,
    tables: &[(String, Rc<Table>)],
    frontier: Option<&finish::partition::Partition>,
) -> bool {
    verify(exec, run, tables, frontier, None).is_ok()
}

/// Applies only when every emit is an antijoin and its *left input* is proven
/// empty for this file. A different output may independently demand its right.
pub(crate) fn certified_empty_left(
    exec: &mut Exec<'_>, run: &mut Run, tables: &[(String, Rc<Table>)],
) -> R<()> {
    let plan = exec.plan;
    let mut demanded = vec![false; plan.nodes.len()];
    let mut pending = Vec::new();
    for (_, output, _, _) in &plan.emits {
        let frame = exec.frames[*output];
        let left = exec.arg_rel(*output, "left")?;
        if frame.op != finish::frame::Op::Antijoin || frame.gate as usize != left {
            return Err("emit has no empty-left antijoin gate".into());
        }
        pending.push(left);
    }
    while let Some(step) = pending.pop() {
        if !std::mem::replace(&mut demanded[step], true) {
            pending.extend(plan.nodes[step].inputs.iter().copied());
        }
    }
    for (_, output, _, _) in &plan.emits { demanded[*output] = true; }
    verify(exec, run, tables, None, Some(&demanded))
}

fn verify(
    exec: &mut Exec<'_>, run: &mut Run, tables: &[(String, Rc<Table>)],
    frontier: Option<&finish::partition::Partition>, demanded: Option<&[bool]>,
) -> R<()> {
    for (name, table) in tables {
        if run
            .catalog(name)
            .is_none_or(|(_, n, t)| n != table.names || t != table.tys)
        {
            let cols: Vec<_> = table
                .names
                .iter()
                .zip(table.tys.iter())
                .map(|(n, &t)| (n.as_str(), t))
                .collect();
            run.declare(name, &cols, finish::Supplier::Host);
        }
    }
    let check = |exec: &mut Exec<'_>, run: &mut Run| -> R<()> {
        let plan = exec.plan;
        for (i, step) in plan.nodes.iter().enumerate() {
            if frontier.is_some_and(|cut| !cut.file_steps[i]) || demanded.is_some_and(|read| !read[i]) {
                continue;
            }
            let _schema = exec.schema(run, i)?;
            let arg_column = |ex: &mut Exec<'_>, run: &mut Run, input, name: &str| -> R<()> {
                let source = ex.arg_rel(i, input)?;
                finish::pos(&ex.schema(run, source)?.names, name).map(|_| ())
            };
            match step.op.as_str() {
                "base" => {
                    let name = step.base().ok_or("base name")?;
                    if matches!(run.supplier(name), finish::Supplier::Host)
                        && finish::host::host_len(name).is_none()
                        && !tables.iter().any(|(bound, _)| bound == name)
                    {
                        // An independently fallible host read still belongs to the request.
                        return Err("host read needs evaluation".into());
                    }
                }
                "rows" | "project" | "extend" | "closure" | "follow" => {}
                "select" => {
                    let select = exec.sel(run, i)?;
                    for e in select
                        .conj
                        .iter()
                        .chain(select.items.iter().map(|(_, e)| e))
                    {
                        if !total(e) {
                            return Err("partial scalar".into());
                        }
                    }
                }
                "group-count" => {
                    if let Some(name) = step.args()["distinct"].as_str() {
                        arg_column(exec, run, "from", name)?;
                    }
                }
                "axis" => {
                    arg_column(exec, run, "from", "id")?;
                    if exec.arg_rel(i, "domain").is_ok() {
                        arg_column(exec, run, "domain", "id")?;
                    }
                    match step.args()["direction"].as_str() {
                        Some(
                            "child" | "parent" | "ancestor" | "descendant" | "nearest-ancestor",
                        ) => {}
                        Some("next-sibling" | "previous-sibling") => {
                            let rel = exec.arg_rel(i, "relation")?;
                            if plan.nodes[rel]
                                .base()
                                .and_then(|n| finish::native_of(run, n))
                                != Some(finish::Native::Child)
                            {
                                return Err(
                                    "sibling order requires concrete ordered children".into()
                                );
                            }
                        }
                        _ => return Err("unverified axis direction".into()),
                    }
                    arg_column(exec, run, "relation", "parent")?;
                    arg_column(exec, run, "relation", "child")?;
                }
                "antijoin" => {
                    let _ = finish::antijoin_kernel::prep(exec, run, i)?;
                }
                "union" => {
                    let _ = finish::union_kernel::prep(exec, run, i)?;
                }
                "capture" | "span-bounds" => {
                    let name = if step.op == "capture" { "path" } else { "span" };
                    let input = exec.arg_rel(i, "from")?;
                    let columns = exec.schema(run, input)?;
                    let column =
                        finish::pos(&columns.names, step.args()[name].as_str().ok_or("column")?)?;
                    if step.op == "capture" && !matches!(columns.tys[column], Ty::Sym | Ty::File)
                        || step.op == "span-bounds" && columns.tys[column] != Ty::Span
                    {
                        return Err("unverified input column".into());
                    }
                }
                _ => return Err("file operator without an empty-answer certificate".into()),
            }
        }
        if frontier.is_some() {
            return Ok(());
        }
        for (_, i, subject, keys) in &plan.emits {
            let schema = exec.schema(run, *i)?;
            for name in std::iter::once(subject).chain(keys) {
                finish::pos(&schema.names, name)?;
            }
        }
        Ok(())
    };
    check(exec, run)
}

/// Preserve the frontier schema even when no output can contain any rows.
pub(crate) fn empty_frontier(
    exec: &mut Exec<'_>,
    run: &mut Run,
    steps: &[usize],
) -> R<Vec<(usize, Rc<Table>)>> {
    steps
        .iter()
        .map(|&step| {
            let schema = exec.schema(run, step)?;
            Ok((
                step,
                Rc::new(Table::with_rows(
                    schema.names.clone(),
                    schema.tys.clone(),
                    0,
                )),
            ))
        })
        .collect()
}
