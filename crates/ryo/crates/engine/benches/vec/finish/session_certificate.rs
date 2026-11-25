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
                "starts_with" | "ends_with" | "float_text_gte" => args.len() == 2,
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

/// Physical span columns whose bounds must be checked on this file before
/// an absence proof can replace execution. Never reuse a file's bounds test
/// on the next version.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum SpanInput {
    Tree,
    Packet(ryo::languages::Relation, usize),
}

pub(crate) type Checked = Option<Result<Box<[SpanInput]>, ()>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Edge { Start, End }

/// Resolve a projected byte offset to an endpoint in this input row. Keep
/// the join operand as part of its identity: a self-join has two rows.
fn offset_origin(
    exec: &mut Exec<'_>, run: &mut Run, join: usize, value: &E, edge: Option<Edge>,
) -> Option<(usize, usize, usize, Edge)> {
    let E::Col(operand, column, _) = *value else { return None };
    let mut node = exec.plan.nodes[join].inputs[operand];
    let mut column = column;
    let mut edge = edge;
    loop {
        let step = &exec.plan.nodes[node];
        if !matches!(step.op.as_str(), "span-bounds" | "source-matches") {
            return Some((operand, node, column, edge?));
        }
        let source = exec.arg_rel(node, "from").ok()?;
        let input = exec.schema(run, source).ok()?;
        if column >= input.tys.len() {
            let output = column - input.tys.len();
            edge = Some(match (step.op.as_str(), output, edge) {
                ("span-bounds", 0, None) => Edge::Start,
                ("span-bounds", 1, None) => Edge::End,
                ("source-matches", 0, Some(_)) => match step.args()["pattern"].as_str()? {
                    r"\A" => Edge::Start,
                    r"\z" => Edge::End,
                    _ => return None,
                },
                _ => return None,
            });
            column = finish::pos(&input.names, step.args()["span"].as_str()?).ok()?;
        }
        node = source;
    }
}

/// A composed span is bounded when both inputs are bounded and the select
/// producing the pair proves their endpoints are in ascending order.
fn ordered_span(exec: &mut Exec<'_>, run: &mut Run, node: usize, start: usize, end: usize) -> R<bool> {
    if start == end { return Ok(true); }
    if exec.plan.nodes[node].op != "select" { return Ok(false); }
    let select = exec.sel(run, node)?;
    let Some(lo) = offset_origin(exec, run, node, &select.items[start].1, Some(Edge::Start)) else { return Ok(false) };
    let Some(hi) = offset_origin(exec, run, node, &select.items[end].1, Some(Edge::End)) else { return Ok(false) };
    if lo == hi { return Ok(true); }
    Ok(select.conj.iter().any(|condition| {
        let E::Cmp(op, left, right) = condition else { return false };
        let a = offset_origin(exec, run, node, left, None);
        let b = offset_origin(exec, run, node, right, None);
        (matches!(op, finish::Op::Lt | finish::Op::Le | finish::Op::Eq) && a == Some(lo) && b == Some(hi))
            || (matches!(op, finish::Op::Gt | finish::Op::Ge | finish::Op::Eq) && a == Some(hi) && b == Some(lo))
    }))
}

/// Trace a span through the same prepared projections used by the operator.
/// A newly composed span needs a separate ordering proof, even when both
/// endpoint spans were valid on their own.
fn span_inputs(
    exec: &mut Exec<'_>, run: &mut Run, node: usize, column: usize, inputs: &mut Vec<SpanInput>, tables: &[(String, Rc<Table>)],
) -> R<()> {
    use ryo_relations::domain::{labels::types::NameRef, relations::service};
    let mut pending = vec![(node, column)];
    let mut visited = std::collections::HashSet::new();
    while let Some((node, column)) = pending.pop() {
        if !visited.insert((node, column)) { continue; }
        let step = &exec.plan.nodes[node];
        let schema = exec.schema(run, node)?;
        if schema.tys.get(column) != Some(&Ty::Span) { return Err("source needs a span".into()); }
        if let Some(name) = step.base() {
            if tables.iter().any(|(hosted, _)| hosted == name) || !matches!(run.supplier(name), finish::Supplier::Provider) {
                return Err("source span has no file reader".into());
            }
            let input = match finish::native_of(run, name) {
                Some(finish::Native::Tree | finish::Native::Location | finish::Native::Token) => SpanInput::Tree,
                _ => SpanInput::Packet(service::relation_named(NameRef(name)).ok_or("undeclared source span")?, column),
            };
            if !inputs.contains(&input) { inputs.push(input); }
            continue;
        }
        match step.op.as_str() {
            "select" => {
                let select = exec.sel(run, node)?;
                let E::Col(source, index, Ty::Span) = select.items[column].1 else {
                    return Err("computed source span".into());
                };
                pending.push((step.inputs[source], index));
            }
            "union" => {
                let left = exec.arg_rel(node, "left")?;
                let right = exec.arg_rel(node, "right")?;
                let left_index = finish::pos(&exec.schema(run, left)?.names, &schema.names[column])?;
                let right_index = finish::pos(&exec.schema(run, right)?.names, &schema.names[column])?;
                pending.extend([(left, left_index), (right, right_index)]);
            }
            "antijoin" => pending.push((exec.arg_rel(node, "left")?, column)),
            "group-min" | "group-max" => pending.push((exec.arg_rel(node, "from")?, column)),
            "project" => {
                let source = exec.arg_rel(node, "from")?;
                let index = finish::pos(&exec.schema(run, source)?.names, &schema.names[column])?;
                pending.push((source, index));
            }
            "source-matches" | "span-bounds" | "span-between" | "replace" | "capture" => {
                let source = exec.arg_rel(node, "from")?;
                let incoming = exec.schema(run, source)?;
                if column < incoming.tys.len() {
                    pending.push((source, column));
                } else if step.op == "source-matches" {
                    let index = finish::pos(&incoming.names, finish::s(&step.args()["span"])?)?;
                    pending.push((source, index));
                } else if step.op == "span-between" {
                    let start = finish::pos(&incoming.names, finish::s(&step.args()["start"])?)?;
                    let end = finish::pos(&incoming.names, finish::s(&step.args()["end"])?)?;
                    if !ordered_span(exec, run, source, start, end)? { return Err("unordered span endpoints".into()); }
                    pending.extend([(source, start), (source, end)]);
                } else {
                    return Err("new span without ordered endpoints".into());
                }
            }
            _ => return Err("unverified source span".into()),
        }
    }
    Ok(())
}

/// A missing witness can skip evaluation once the binding has passed these checks.
pub(crate) fn certified_empty(
    exec: &mut Exec<'_>,
    run: &mut Run,
    tables: &[(String, Rc<Table>)],
    frontier: Option<&finish::partition::Partition>,
) -> R<Box<[SpanInput]>> {
    verify(exec, run, tables, frontier, None)
}

/// Applies only when every emit is an antijoin and its *left input* is proven
/// empty for this file. A different output may independently demand its right.
pub(crate) fn certified_empty_left(
    exec: &mut Exec<'_>, run: &mut Run, tables: &[(String, Rc<Table>)],
) -> R<Box<[SpanInput]>> {
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
) -> R<Box<[SpanInput]>> {
    let mut spans = Vec::new();
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
    let timed = exec.timing;
    let mut at = None;
    let mut check = |exec: &mut Exec<'_>, run: &mut Run| -> R<()> {
        let plan = exec.plan;
        for (i, step) in plan.nodes.iter().enumerate() {
            if frontier.is_some_and(|cut| !cut.file_steps[i]) || demanded.is_some_and(|read| !read[i]) {
                continue;
            }
            at = Some(i);
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
                            return Err(if timed { format!("partial scalar {e:?}") } else { "partial scalar".into() });
                        }
                    }
                }
                "group-min" | "group-max" => {
                    for name in finish::strs(&step.args()["keys"])? {
                        arg_column(exec, run, "from", name)?;
                    }
                    arg_column(exec, run, "from", finish::s(&step.args()["order"])?)?;
                }
                "source-matches" => {
                    let source = exec.arg_rel(i, "from")?;
                    let schema = exec.schema(run, source)?;
                    let span = finish::pos(&schema.names, finish::s(&step.args()["span"])?)?;
                    if step.args().get("base-line").is_some() {
                        return Err("source match needs line arithmetic".into());
                    }
                    let _ = run.re(finish::s(&step.args()["pattern"])?)?;
                    span_inputs(exec, run, source, span, &mut spans, tables)?;
                }
                "replace" => {
                    let source = exec.arg_rel(i, "from")?;
                    let input = exec.schema(run, source)?;
                    let text = finish::pos(&input.names, finish::s(&step.args()["text"])?)?;
                    if input.tys[text] != Ty::Sym { return Err("replace input is not text".into()); }
                    let _ = finish::s(&step.args()["replacement"])?;
                    let _ = run.re(finish::s(&step.args()["pattern"])?)?;
                }
                "span-between" => {
                    for argument in ["start", "end"] {
                        arg_column(exec, run, "from", finish::s(&step.args()[argument])?)?;
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
            at = Some(*i);
            let schema = exec.schema(run, *i)?;
            for name in std::iter::once(subject).chain(keys) {
                finish::pos(&schema.names, name)?;
            }
        }
        Ok(())
    };
    let result = check(exec, run);
    if exec.timing && let Err(reason) = &result && let Some(i) = at {
        eprintln!("PAIR_CERTIFICATE_REFUSAL\t{i}\t{}\t{reason}", exec.plan.nodes[i].op);
    }
    result.map(|()| spans.into_boxed_slice())
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
