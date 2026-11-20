//! The `owner_facts` service: the scope each node is evaluated in, by Python's rules. A definition's decorators, defaults,
//! annotations and bases are evaluated around it, only its statements and its type parameters inside it; a lambda's parameters
//! around it, its body inside; the first iterable of a comprehension around the comprehension, every other part inside it.

use super::types::{Evaluated, Owners, Readiness};
use crate::domain::node::service::{id_of, node_at};
use crate::domain::node::types::SyntaxFile;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{Position, node_table};

/// Where a child of `above` is evaluated: inside the scope `above` opens, or around `above`. The statements and the type
/// parameters of a definition, the body of a lambda and every part of a module and of a comprehension are inside; every other
/// child, and every child of a node that opens no scope, is around.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the nodes that open a scope are named; a child of any other node is evaluated where that node is"
)]
const fn evaluated(above: AnyNodeRef<'_>, child: AnyNodeRef<'_>) -> Evaluated {
    let inside = match above {
        AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::StmtClassDef(_) => {
            child.is_statement() || matches!(child, AnyNodeRef::TypeParams(_))
        }
        AnyNodeRef::ExprLambda(_) => child.is_expression(),
        AnyNodeRef::ModModule(_)
        | AnyNodeRef::ModExpression(_)
        | AnyNodeRef::ExprListComp(_)
        | AnyNodeRef::ExprSetComp(_)
        | AnyNodeRef::ExprDictComp(_)
        | AnyNodeRef::ExprGenerator(_) => true,
        _ => false,
    };
    if inside {
        Evaluated::Inside
    } else {
        Evaluated::Around
    }
}

/// The first `for` clause of a comprehension.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the four comprehensions hold `for` clauses; any other node has no first clause"
)]
fn first_clause(owner: AnyNodeRef<'_>) -> Option<Position> {
    match owner {
        AnyNodeRef::ExprListComp(comprehension) => comprehension.generators.first().and_then(id_of),
        AnyNodeRef::ExprSetComp(comprehension) => comprehension.generators.first().and_then(id_of),
        AnyNodeRef::ExprDictComp(comprehension) => comprehension.generators.first().and_then(id_of),
        AnyNodeRef::ExprGenerator(comprehension) => {
            comprehension.generators.first().and_then(id_of)
        }
        _ => None,
    }
}

/// The comprehension whose first `for` clause is `clause` and whose first iterable is `child`: the one part evaluated around the
/// comprehension. None for any other child of a clause.
fn first_iterable_of(file: &SyntaxFile<'_>, clause: Position, child: Position) -> Option<Position> {
    let AnyNodeRef::Comprehension(typed) = node_at(file, clause)? else {
        return None;
    };
    let owner = node_table::parent_of(file.table, clause);
    (id_of(&typed.iter) == Some(child)
        && node_at(file, owner).and_then(first_clause) == Some(clause))
    .then_some(owner)
}

/// The scope each node of the file is evaluated in, by the node's id: worked out on the first call for a file, read on the next.
pub(crate) fn owners<'o>(file: &SyntaxFile<'_>, kept: &'o mut Owners) -> &'o [Position] {
    if kept.ready == Readiness::Ready {
        return &kept.scopes;
    }
    let root = Position(0);
    kept.scopes.clear();
    kept.scopes.resize(file.nodes.len(), root);
    for (index, node) in file.nodes.iter().enumerate().skip(1) {
        let id = Position(u32::try_from(index).unwrap_or(u32::MAX));
        let parent = node_table::parent_of(file.table, id);
        let around = kept
            .scopes
            .get(usize::from(parent))
            .copied()
            .unwrap_or(root);
        let scope = match (node_at(file, parent), first_iterable_of(file, parent, id)) {
            (_, Some(comprehension)) => kept
                .scopes
                .get(usize::from(comprehension))
                .copied()
                .unwrap_or(root),
            (Some(above), None) if evaluated(above, *node) == Evaluated::Inside => parent,
            (Some(_) | None, None) => around,
        };
        if let Some(slot) = kept.scopes.get_mut(index) {
            *slot = scope;
        }
    }
    kept.ready = Readiness::Ready;
    &kept.scopes
}

/// The owner of one node among the owners of its file.
#[must_use]
pub(crate) fn owner_of(owners: &[Position], id: Position) -> Position {
    owners.get(usize::from(id)).copied().unwrap_or(Position(0))
}
