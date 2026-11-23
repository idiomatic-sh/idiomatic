//! The source expressions that supply ordinary assignments, named expressions and annotated assignments.
use super::ColumnFile;
use crate::domain::node::service::{id_of, table_id};
use ruff_python_ast::{AnyNodeRef, Expr};

/// A tuple or list display supplies its elements in their written order.
fn elements(expression: &Expr) -> Option<&[Expr]> {
    match expression {
        Expr::Tuple(tuple) => Some(&tuple.elts),
        Expr::List(list) => Some(&list.elts),
        _ => None,
    }
}

/// Unpacking sources are statically paired only for equal lengths with no expansion at that level.
fn ordered(target: &Expr, value: &Expr) -> bool {
    let Some(targets) = elements(target) else { return !matches!(target, Expr::Starred(_)) };
    let Some(values) = elements(value) else { return false };
    targets.len() == values.len()
        && !values.iter().any(|expr| matches!(expr, Expr::Starred(_)))
        && targets.iter().zip(values).all(|(left, right)| ordered(left, right))
}

fn pairs<'a>(target: &'a Expr, value: &'a Expr, emit: &mut impl FnMut(&'a Expr, &'a Expr)) {
    if let (Some(targets), Some(values)) = (elements(target), elements(value)) {
        for (left, right) in targets.iter().zip(values) {
            pairs(left, right, emit);
        }
    } else if matches!(target, Expr::Name(_)) {
        emit(target, value);
    }
}

impl ColumnFile<'_, '_> {
    /// A name gets the RHS value; fixed tuple/list displays supply corresponding unpacked elements.
    /// An unresolved pattern records its original target and complete RHS, with no inferred element value.
    pub fn visit_assignment_sources(&self, mut visit: impl FnMut(u32, u32, &'static str, &'static str)) {
        let tree = self.file.syntax.tree;
        let mut assignment = |target: &Expr, value: &Expr, operation| {
            let mut emit = |left: &Expr, right: &Expr, association| {
                if let (Some(left), Some(right)) = (id_of(left), id_of(right)) {
                    visit(table_id(tree.segments, left).0, table_id(tree.segments, right).0, operation, association);
                }
            };
            if matches!(target, Expr::Name(_)) {
                emit(target, value, "direct");
            } else if elements(target).is_some() && ordered(target, value) {
                pairs(target, value, &mut |left, right| emit(left, right, "element"));
            } else {
                emit(target, value, "pattern");
            }
        };
        for node in tree.nodes {
            match node {
                AnyNodeRef::StmtAssign(s) => {
                    for target in &s.targets {
                        assignment(target, &s.value, "assignment");
                    }
                }
                AnyNodeRef::ExprNamed(s) => assignment(&s.target, &s.value, "named"),
                AnyNodeRef::StmtAnnAssign(s) => {
                    if let Some(value) = &s.value {
                        assignment(&s.target, value, "annotated");
                    }
                }
                _ => {}
            }
        }
    }
}
