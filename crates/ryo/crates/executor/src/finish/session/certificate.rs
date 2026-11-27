//! Executor-side emptiness helpers: scalar totality and the empty frontier.
use super::super as finish;
use finish::{E, Exec, R, Run, Table};
use std::rc::Rc;

// Dynamic pattern compilation can refuse a non-empty intermediate even
// when the downstream emit is empty. A literal pattern is already E::TextTest.
pub(crate) fn total(expr: &E) -> bool {
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
